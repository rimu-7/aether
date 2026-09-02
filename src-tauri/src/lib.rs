use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo::{System, Networks, Disks};

use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

pub mod commands;
pub mod models;
pub mod platform;
pub mod services;

#[derive(Debug, Clone)]
pub struct MenubarState {
    pub enabled: bool,
}

pub struct AppState {
    pub sys: Mutex<System>,
    pub networks: Mutex<Networks>,
    pub disks: Mutex<Disks>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Plugins
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            // ============================================================
            // GLOBAL MENUBAR / SPEED METER STATE
            // ============================================================

            let menubar_state = Arc::new(Mutex::new(MenubarState { enabled: false }));
            app.manage(menubar_state.clone());

            let app_state = Arc::new(AppState {
                sys: Mutex::new(System::new_all()),
                networks: Mutex::new(Networks::new_with_refreshed_list()),
                disks: Mutex::new(Disks::new_with_refreshed_list()),
            });
            app.manage(app_state.clone());

            // ============================================================
            // TRAY ICON (Internet Speed Meter)
            // ============================================================

            let _tray = TrayIconBuilder::with_id("speed")
                .title("↓ 0B ↑ 0B")
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        let app = tray.app_handle();

                        if let Some(window) = app.get_webview_window("main") {
                            if let Err(error) = window.show() {
                                eprintln!("[Menubar] Failed to show window: {:?}", error);
                            }

                            if let Err(error) = window.set_focus() {
                                eprintln!("[Menubar] Failed to focus window: {:?}", error);
                            }
                        }
                    }

                    _ => {}
                })
                .build(app)?;

            // On Linux, the tray may not be available without a system tray manager.
            // Log the failure but don't crash; the speed meter still works in the dashboard.
            if cfg!(target_os = "linux") {
                if let Err(error) = _tray.set_visible(false) {
                    eprintln!("[Menubar] Linux tray icon not available (no system tray?): {:?}", error);
                    eprintln!("[Menubar] Speed meter will still work in the Dashboard.");
                }
            } else {
                if let Err(error) = _tray.set_visible(false) {
                    eprintln!("[Menubar] Failed to hide initial tray: {:?}", error);
                }
            }

            println!("[Menubar] Tray initialized successfully");

            // ============================================================
            // NETWORK SPEED MONITOR
            // ============================================================

            let app_handle = app.handle().clone();
            let state_clone = app_state.clone();

            tauri::async_runtime::spawn(async move {
                println!("[Menubar] Network monitor started");

                let (mut last_rx, mut last_tx) = {
                    let networks = state_clone.networks.lock().unwrap();
                    let rx: u64 = networks.iter().map(|(_, network)| network.total_received()).sum();
                    let tx: u64 = networks.iter().map(|(_, network)| network.total_transmitted()).sum();
                    (rx, tx)
                };

                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;

                    let (current_rx, current_tx) = {
                        let mut networks = match state_clone.networks.lock() {
                            Ok(n) => n,
                            Err(_) => continue,
                        };
                        networks.refresh(true);
                        let rx: u64 = networks.iter().map(|(_, network)| network.total_received()).sum();
                        let tx: u64 = networks.iter().map(|(_, network)| network.total_transmitted()).sum();
                        (rx, tx)
                    };

                    let rx_speed = current_rx.saturating_sub(last_rx);
                    let tx_speed = current_tx.saturating_sub(last_tx);

                    last_rx = current_rx;
                    last_tx = current_tx;

                    // Read current state.
                    let enabled = match menubar_state.lock() {
                        Ok(state) => state.enabled,
                        Err(error) => {
                            eprintln!("[Menubar] Failed to lock state: {:?}", error);
                            continue;
                        }
                    };

                    // Do not update the tray while disabled.
                    if !enabled {
                        continue;
                    }

                    // Find our tray item.
                    let Some(tray) = app_handle.tray_by_id("speed") else {
                        eprintln!("[Menubar] Tray 'speed' not found");
                        continue;
                    };

                    let title =
                        format!("↓ {} ↑ {}", format_speed(rx_speed), format_speed(tx_speed));

                    if let Err(error) = tray.set_title(Some(title.clone())) {
                        // On some Linux tray implementations, set_title may not be supported.
                        // This is non-fatal; the dashboard still works.
                        if cfg!(target_os = "linux") {
                            eprintln!("[Menubar] set_title not supported by this tray (Linux): {:?}", error);
                        } else {
                            eprintln!("[Menubar] Failed to update title: {:?}", error);
                        }
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Hide instead of actually closing.
                if let Err(error) = window.hide() {
                    eprintln!("[Window] Failed to hide window: {:?}", error);
                }

                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_system_info,
            commands::applications::scan_applications,
            commands::applications::get_application_artifacts,
            commands::applications::get_app_icon,
            commands::applications::delete_artifacts,
            commands::packages::get_installed_packages,
            commands::packages::uninstall_package,
            commands::cleaner::scan_cleanable_items,
            commands::cleaner::delete_cleanable_items,
            commands::cleaner::reveal_in_finder,
            commands::files::scan_large_files,
            commands::files::delete_files,
            update_menubar_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Aether");
}

// ========================================================================
// MENUBAR SETTINGS COMMAND
// ========================================================================

#[tauri::command]
fn update_menubar_settings(
    enabled: bool,
    state: tauri::State<'_, Arc<Mutex<MenubarState>>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    println!(
        "[Menubar] update_menubar_settings called: enabled={}",
        enabled
    );

    // ------------------------------------------------------------
    // Update state
    // ------------------------------------------------------------

    {
        let mut menubar_state = state
            .lock()
            .map_err(|_| "[Menubar] Failed to acquire state lock".to_string())?;

        menubar_state.enabled = enabled;
    }

    println!("[Menubar] State updated successfully: enabled={}", enabled);

    // ------------------------------------------------------------
    // Find tray and change visibility
    // ------------------------------------------------------------
    // On macOS and Windows, the tray is expected to be available.
    // On Linux, the tray requires a system tray manager; if it's not
    // available, we log a warning but still return Ok so the dashboard
    // speed meter toggle works.
    match app.tray_by_id("speed") {
        Some(tray) => {
            if let Err(error) = tray.set_visible(enabled) {
                eprintln!("[Menubar] WARNING: Could not set tray visibility: {:?}", error);
                if cfg!(target_os = "linux") {
                    eprintln!("[Menubar] System tray may not be available on this Linux system.");
                }
            } else {
                println!("[Menubar] Tray visibility successfully set to {}", enabled);
            }
        }
        None => {
            if cfg!(target_os = "linux") {
                eprintln!("[Menubar] System tray not available on this Linux system (no tray manager).");
                println!("[Menubar] Speed meter state stored; dashboard will still show network speed.");
            } else {
                eprintln!("[Menubar] ERROR: Tray 'speed' does not exist");
                return Err("Tray 'speed' was not found".to_string());
            }
        }
    }

    Ok(())
}

// ========================================================================
// SPEED FORMATTER
// ========================================================================

fn format_speed(bytes_per_second: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    let bytes = bytes_per_second as f64;

    if bytes < KB {
        format!("{}B", bytes_per_second)
    } else if bytes < MB {
        format!("{:.1}K", bytes / KB)
    } else if bytes < GB {
        format!("{:.1}M", bytes / MB)
    } else {
        format!("{:.1}G", bytes / GB)
    }
}
