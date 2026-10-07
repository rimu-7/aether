pub mod utils;

#[cfg(target_os = "windows")]
pub mod windows_registry;

pub use utils::*;
