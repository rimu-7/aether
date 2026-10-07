use crate::models::package::Package;
use crate::services::packages;

#[tauri::command]
pub async fn get_installed_packages() -> Result<Vec<Package>, String> {
    tauri::async_runtime::spawn_blocking(packages::get_installed_packages)
        .await
        .map_err(|error| format!("Package scan did not complete: {error}"))?
}

#[tauri::command]
pub async fn uninstall_package(
    id: String,
    is_cask: bool,
    manager: Option<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        packages::uninstall_package(&id, is_cask, manager.as_deref())
    })
    .await
    .map_err(|error| format!("Package uninstall did not complete: {error}"))?
}
