use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

pub mod commands;
pub mod models;
pub mod services;

#[derive(Debug, Clone)]
pub struct MenubarState {
    pub enabled: bool,
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
            // GLOBAL MENUBAR STATE
            // ============================================================

            let menubar_state = Arc::new(Mutex::new(MenubarState { enabled: false }));

            app.manage(menubar_state.clone());

            let tray = TrayIconBuilder::with_id("speed")
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

            // Start visible for testing!
            if let Err(error) = tray.set_visible(true) {
                eprintln!("[Menubar] Failed to show initial tray: {:?}", error);
            }

            println!("[Menubar] Tray initialized successfully");

            // ============================================================
            // NETWORK SPEED MONITOR
            // ============================================================

            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                use sysinfo::Networks;

                println!("[Menubar] Network monitor started");

                let mut networks = Networks::new_with_refreshed_list();

                let mut last_rx: u64 = networks
                    .iter()
                    .map(|(_, network)| network.total_received())
                    .sum();

                let mut last_tx: u64 = networks
                    .iter()
                    .map(|(_, network)| network.total_transmitted())
                    .sum();

                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;

                    // Refresh network statistics.
                    networks.refresh(true);

                    let current_rx: u64 = networks
                        .iter()
                        .map(|(_, network)| network.total_received())
                        .sum();

                    let current_tx: u64 = networks
                        .iter()
                        .map(|(_, network)| network.total_transmitted())
                        .sum();

                    // Calculate bytes received/transmitted during
                    // approximately the previous one-second interval.
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
                        eprintln!("[Menubar] Failed to update title: {:?}", error);
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
    // Find tray
    // ------------------------------------------------------------

    let tray = match app.tray_by_id("speed") {
        Some(tray) => {
            println!("[Menubar] Tray 'speed' found");
            tray
        }

        None => {
            eprintln!("[Menubar] ERROR: Tray 'speed' does not exist");

            return Err("Tray 'speed' was not found".to_string());
        }
    };

    // ------------------------------------------------------------
    // Change visibility
    // ------------------------------------------------------------

    match tray.set_visible(enabled) {
        Ok(_) => {
            println!("[Menubar] Tray visibility successfully set to {}", enabled);
        }

        Err(error) => {
            eprintln!("[Menubar] ERROR setting tray visibility: {:?}", error);

            return Err(format!("Failed to set tray visibility: {:?}", error));
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
