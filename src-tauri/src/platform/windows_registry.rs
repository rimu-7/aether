//! Windows installed-program registry access.
//!
//! Keep this behind a Windows-only module so the rest of the application can
//! share a single, native source of truth without invoking PowerShell.

use std::collections::HashSet;

use winreg::enums::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
};
use winreg::RegKey;

const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

#[derive(Debug, Clone)]
pub struct InstalledProgram {
    /// Stable identifier containing the registry hive/view and subkey name.
    /// Display names are not unique, so they must never be used for uninstall.
    pub id: String,
    pub display_name: String,
    pub display_version: Option<String>,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    pub display_icon: Option<String>,
    pub uninstall_string: Option<String>,
    pub quiet_uninstall_string: Option<String>,
    pub estimated_size_bytes: Option<u64>,
    pub is_system_component: bool,
}

#[derive(Clone, Copy)]
enum RegistryRoot {
    LocalMachine64,
    LocalMachine32,
    CurrentUser,
}

impl RegistryRoot {
    fn id_prefix(self) -> &'static str {
        match self {
            Self::LocalMachine64 => "HKLM64",
            Self::LocalMachine32 => "HKLM32",
            Self::CurrentUser => "HKCU",
        }
    }

    fn uninstall_key(self) -> std::io::Result<RegKey> {
        match self {
            Self::LocalMachine64 => RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | KEY_WOW64_64KEY),
            Self::LocalMachine32 => RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | KEY_WOW64_32KEY),
            Self::CurrentUser => {
                RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(UNINSTALL_KEY, KEY_READ)
            }
        }
    }
}

fn non_empty_string(key: &RegKey, value_name: &str) -> Option<String> {
    key.get_value::<String, _>(value_name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn is_system_component(key: &RegKey) -> bool {
    key.get_value::<u32, _>("SystemComponent")
        .map(|value| value == 1)
        .unwrap_or(false)
}

/// Enumerate applications registered in Add/Remove Programs.  Access errors
/// for one hive/view are intentionally ignored: a standard user commonly
/// cannot read every optional subkey, but the accessible inventory is still
/// useful and complete for that user.
pub fn installed_programs() -> Vec<InstalledProgram> {
    let mut programs = Vec::new();
    let mut seen_ids = HashSet::new();

    for root in [
        RegistryRoot::LocalMachine64,
        RegistryRoot::LocalMachine32,
        RegistryRoot::CurrentUser,
    ] {
        let Ok(uninstall) = root.uninstall_key() else {
            continue;
        };

        for key_name in uninstall.enum_keys().filter_map(Result::ok) {
            let Ok(key) = uninstall.open_subkey(&key_name) else {
                continue;
            };

            let Some(display_name) = non_empty_string(&key, "DisplayName") else {
                continue;
            };

            let id = format!("{}:{}", root.id_prefix(), key_name);
            if !seen_ids.insert(id.clone()) {
                continue;
            }

            programs.push(InstalledProgram {
                id,
                display_name,
                display_version: non_empty_string(&key, "DisplayVersion"),
                publisher: non_empty_string(&key, "Publisher"),
                install_location: non_empty_string(&key, "InstallLocation"),
                display_icon: non_empty_string(&key, "DisplayIcon"),
                uninstall_string: non_empty_string(&key, "UninstallString"),
                quiet_uninstall_string: non_empty_string(&key, "QuietUninstallString"),
                // Windows stores EstimatedSize in KiB.
                estimated_size_bytes: key
                    .get_value::<u32, _>("EstimatedSize")
                    .ok()
                    .map(|size_kib| u64::from(size_kib) * 1024),
                is_system_component: is_system_component(&key),
            });
        }
    }

    programs.sort_by(|left, right| {
        left.display_name
            .cmp(&right.display_name)
            .then_with(|| left.id.cmp(&right.id))
    });
    programs
}

pub fn find_installed_program(id: &str) -> Option<InstalledProgram> {
    installed_programs()
        .into_iter()
        .find(|program| program.id == id)
}
