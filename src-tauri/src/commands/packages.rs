use crate::models::package::Package;
use crate::services::packages;

#[tauri::command]
pub async fn get_installed_packages() -> Result<Vec<Package>, String> {
    // We could offload this to a separate thread since brew info can take a second, 
    // but async tauri commands run on a threadpool anyway.
    packages::get_installed_packages()
}

#[tauri::command]
pub async fn uninstall_package(id: String, is_cask: bool) -> Result<(), String> {
    packages::uninstall_package(&id, is_cask)
}
