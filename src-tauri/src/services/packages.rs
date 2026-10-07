use std::collections::HashSet;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::path::Path;
use std::path::PathBuf;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::Output;

#[cfg(target_os = "macos")]
use std::fs;

use crate::models::package::Package;
use crate::platform::utils::background_command;

#[cfg(target_os = "windows")]
use crate::platform::windows_registry;

fn package(
    id: impl Into<String>,
    name: impl Into<String>,
    description: Option<String>,
    version: impl Into<String>,
    is_cask: bool,
    manager: impl Into<String>,
    size_bytes: u64,
) -> Package {
    Package {
        id: id.into(),
        name: name.into(),
        description,
        version: version.into(),
        is_cask,
        manager: manager.into(),
        size_bytes,
    }
}

/// Look up an executable without relying only on the stripped PATH inherited
/// by a macOS/Linux GUI app. This covers standard package-manager locations
/// while still honoring an explicitly supplied PATH.
fn find_command(command: &str) -> Option<PathBuf> {
    let direct = PathBuf::from(command);
    if direct.components().count() > 1 && direct.is_file() {
        return Some(direct);
    }

    let mut search_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();

    search_dirs.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/local/Homebrew/bin"),
        PathBuf::from("/opt/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
        PathBuf::from("/usr/sbin"),
        PathBuf::from("/sbin"),
    ]);

    if let Some(home) = dirs::home_dir() {
        search_dirs.push(home.join(".local").join("bin"));
        search_dirs.push(home.join("bin"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(system_root) = std::env::var_os("SystemRoot") {
            search_dirs.push(PathBuf::from(system_root).join("System32"));
        }
    }

    let mut seen = HashSet::new();
    for directory in search_dirs {
        if !seen.insert(directory.clone()) {
            continue;
        }

        let candidate = directory.join(command);
        if candidate.is_file() {
            return Some(candidate);
        }

        #[cfg(target_os = "windows")]
        {
            let candidate = directory.join(format!("{command}.exe"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn run_command(program: &Path, args: &[&str], label: &str) -> Result<Output, String> {
    let output = background_command(program)
        .args(args)
        .output()
        .map_err(|error| format!("Could not run {label}: {error}"))?;

    if output.status.success() {
        return Ok(output);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let detail = if !stderr.is_empty() { stderr } else { stdout };
    Err(format!(
        "{label} exited with {}{}",
        output.status,
        if detail.is_empty() {
            String::new()
        } else {
            format!(": {detail}")
        }
    ))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn output_lines(output: Output) -> impl Iterator<Item = String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>()
        .into_iter()
}

#[cfg(target_os = "macos")]
fn brew_path() -> Result<PathBuf, String> {
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin/brew"),
        PathBuf::from("/usr/local/bin/brew"),
        PathBuf::from("/usr/local/Homebrew/bin/brew"),
    ];

    if let Some(prefix) = std::env::var_os("HOMEBREW_PREFIX") {
        candidates.insert(0, PathBuf::from(prefix).join("bin").join("brew"));
    }

    if let Some(path) = candidates.into_iter().find(|path| path.is_file()) {
        return Ok(path);
    }

    if let Some(path) = find_command("brew") {
        return Ok(path);
    }

    // A GUI app commonly starts with a minimal PATH. Ask the user's login
    // shell as a last resort so custom Homebrew prefixes still work.
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| "/bin/zsh".into());
    for shell in [
        PathBuf::from(&shell),
        PathBuf::from("/bin/zsh"),
        PathBuf::from("/bin/bash"),
    ] {
        if !shell.is_file() {
            continue;
        }

        let Ok(output) = background_command(&shell)
            .args(["-lc", "command -v brew"])
            .output()
        else {
            continue;
        };

        if !output.status.success() {
            continue;
        }

        if let Some(path) = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(PathBuf::from)
            .filter(|path| path.is_file())
        {
            return Ok(path);
        }
    }

    Err(
        "Homebrew was not found. Install Homebrew, or ensure its bin directory is available to your login shell."
            .to_string(),
    )
}

#[cfg(target_os = "macos")]
fn parse_brew_list(output: Output, is_cask: bool) -> Vec<Package> {
    output_lines(output)
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;
            let version = parts.collect::<Vec<_>>().join(", ");
            Some(package(
                format!("homebrew:{name}"),
                name,
                None,
                if version.is_empty() {
                    "Unknown"
                } else {
                    &version
                },
                is_cask,
                "homebrew",
                0,
            ))
        })
        .collect()
}

/// Homebrew refuses to list every cask when one third-party tap has not been
/// trusted yet. The already-installed casks are still recorded in Caskroom,
/// so use that local inventory as a safe read-only fallback instead of hiding
/// all packages or asking the user to trust a tap.
#[cfg(target_os = "macos")]
fn installed_brew_casks_from_disk(brew: &Path) -> Vec<Package> {
    let prefix = run_command(brew, &["--prefix"], "Homebrew prefix")
        .ok()
        .and_then(|output| output_lines(output).next())
        .map(PathBuf::from)
        .or_else(|| brew.parent().and_then(Path::parent).map(PathBuf::from));
    let Some(caskroom) = prefix.map(|prefix| prefix.join("Caskroom")) else {
        return Vec::new();
    };

    let Ok(entries) = fs::read_dir(caskroom) else {
        return Vec::new();
    };

    let mut casks = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            if name.is_empty() || name.starts_with('.') {
                return None;
            }

            let version = fs::read_dir(&path)
                .ok()?
                .flatten()
                .filter_map(|version| {
                    version
                        .file_type()
                        .ok()
                        .filter(|kind| kind.is_dir())
                        .map(|_| version.file_name().to_string_lossy().to_string())
                })
                .filter(|version| !version.starts_with('.'))
                .max()
                .unwrap_or_else(|| "Unknown".to_string());

            Some(package(
                format!("homebrew:{name}"),
                name,
                None,
                version,
                true,
                "homebrew",
                0,
            ))
        })
        .collect::<Vec<_>>();
    casks.sort_by(|left, right| left.name.cmp(&right.name));
    casks
}

#[cfg(target_os = "macos")]
fn get_macos_packages() -> Result<Vec<Package>, String> {
    let brew = brew_path()?;
    let brew_clone = brew.clone();
    let formula_handle = std::thread::spawn(move || {
        run_command(
            &brew_clone,
            &["list", "--formula", "--versions"],
            "Homebrew formula list",
        )
    });
    let casks = run_command(
        &brew,
        &["list", "--cask", "--versions"],
        "Homebrew cask list",
    );
    let formulae = formula_handle
        .join()
        .unwrap_or_else(|_| Err("Formula thread panicked".to_string()));

    // A single untrusted or broken cask must not hide every installed formula.
    // Homebrew exits non-zero for the whole cask listing in that case, so retain
    // whichever inventory source succeeded and only surface an error if neither
    // source is available.
    match (formulae, casks) {
        (Ok(formulae), Ok(casks)) => {
            let mut packages = parse_brew_list(formulae, false);
            packages.extend(parse_brew_list(casks, true));
            Ok(packages)
        }
        (Ok(formulae), Err(cask_error)) => {
            eprintln!(
                "Homebrew cask list failed; reading the installed Caskroom inventory instead: {cask_error}"
            );
            let mut packages = parse_brew_list(formulae, false);
            packages.extend(installed_brew_casks_from_disk(&brew));
            Ok(packages)
        }
        (Err(_), Ok(casks)) => Ok(parse_brew_list(casks, true)),
        (Err(formula_error), Err(cask_error)) => {
            let casks = installed_brew_casks_from_disk(&brew);
            if casks.is_empty() {
                Err(format!(
                    "Could not read Homebrew packages. Formulae: {formula_error}. Casks: {cask_error}"
                ))
            } else {
                eprintln!(
                    "Homebrew package commands failed; reading the installed Caskroom inventory instead. Formulae: {formula_error}. Casks: {cask_error}"
                );
                Ok(casks)
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageManager {
    Dnf,
    Apt,
    Pacman,
    Zypper,
    Emerge,
    Xbps,
}

#[cfg(target_os = "linux")]
impl PackageManager {
    fn detect() -> Option<Self> {
        [
            (Self::Dnf, "dnf"),
            (Self::Apt, "dpkg-query"),
            (Self::Pacman, "pacman"),
            (Self::Zypper, "zypper"),
            (Self::Emerge, "emerge"),
            (Self::Xbps, "xbps-query"),
        ]
        .into_iter()
        .find_map(|(manager, command)| find_command(command).map(|_| manager))
    }

    fn command(self) -> &'static str {
        match self {
            Self::Dnf => "rpm",
            Self::Apt => "dpkg-query",
            Self::Pacman => "pacman",
            Self::Zypper => "rpm",
            Self::Emerge => "qlist",
            Self::Xbps => "xbps-query",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Dnf => "dnf",
            Self::Apt => "apt",
            Self::Pacman => "pacman",
            Self::Zypper => "zypper",
            Self::Emerge => "emerge",
            Self::Xbps => "xbps",
        }
    }
}

#[cfg(target_os = "linux")]
fn parse_linux_records(output: Output, manager: &str, has_size: bool) -> Vec<Package> {
    output_lines(output)
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(if has_size { 4 } else { 3 }, '|').collect();
            if parts.len() < 2 || parts[0].trim().is_empty() {
                return None;
            }

            let size_bytes = if has_size {
                parts
                    .get(3)
                    .and_then(|size| size.trim().parse::<u64>().ok())
                    .unwrap_or(0)
            } else {
                0
            };
            let description = parts
                .get(2)
                .map(|description| description.trim().to_owned())
                .filter(|description| !description.is_empty());
            let name = parts[0].trim();
            Some(package(
                format!("{manager}:{name}"),
                name,
                description,
                parts[1].trim(),
                false,
                manager,
                size_bytes,
            ))
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn get_primary_linux_packages(manager: PackageManager) -> Result<Vec<Package>, String> {
    let command = find_command(manager.command()).ok_or_else(|| {
        format!(
            "{} is no longer available in this desktop session",
            manager.command()
        )
    })?;

    match manager {
        PackageManager::Dnf | PackageManager::Zypper => {
            let output = run_command(
                &command,
                &[
                    "-qa",
                    "--qf",
                    "%{NAME}|%{VERSION}-%{RELEASE}|%{SUMMARY}|%{SIZE}\\n",
                ],
                "RPM package list",
            )?;
            Ok(parse_linux_records(output, manager.name(), true))
        }
        PackageManager::Apt => {
            let output = run_command(
                &command,
                &[
                    "-W",
                    "-f=${Package}|${Version}|${binary:Summary}|${Installed-Size}\\n",
                ],
                "APT package list",
            )?;
            // dpkg reports installed size in KiB.
            Ok(parse_linux_records(output, manager.name(), true)
                .into_iter()
                .map(|mut package| {
                    package.size_bytes = package.size_bytes.saturating_mul(1024);
                    package
                })
                .collect())
        }
        PackageManager::Pacman => {
            let output = run_command(&command, &["-Q"], "Pacman package list")?;
            Ok(output_lines(output)
                .filter_map(|line| {
                    let (name, version) = line.split_once(char::is_whitespace)?;
                    Some(package(
                        format!("pacman:{name}"),
                        name,
                        None,
                        version.trim(),
                        false,
                        "pacman",
                        0,
                    ))
                })
                .collect())
        }
        PackageManager::Emerge => {
            // `qlist` is supplied by app-portage/portage-utils. If it is not
            // installed, return a clear diagnostic instead of an empty list.
            let qlist = find_command("qlist").ok_or_else(|| {
                "Gentoo package listing requires app-portage/portage-utils (qlist).".to_string()
            })?;
            let output = run_command(&qlist, &["-IC"], "Gentoo package list")?;
            Ok(output_lines(output)
                .map(|name| {
                    package(
                        format!("emerge:{name}"),
                        name,
                        None,
                        "Unknown",
                        false,
                        "emerge",
                        0,
                    )
                })
                .collect())
        }
        PackageManager::Xbps => {
            let output = run_command(&command, &["-l"], "XBPS package list")?;
            Ok(output_lines(output)
                .filter_map(|line| {
                    let line = line.trim_start();
                    let rest = line.strip_prefix("ii ").unwrap_or(line);
                    let (name_version, description) = rest.split_once(' ').unwrap_or((rest, ""));
                    let (name, version) = name_version.rsplit_once('-')?;
                    Some(package(
                        format!("xbps:{name}"),
                        name,
                        (!description.trim().is_empty()).then(|| description.trim().to_owned()),
                        version,
                        false,
                        "xbps",
                        0,
                    ))
                })
                .collect())
        }
    }
}

#[cfg(target_os = "linux")]
fn supplemental_linux_packages() -> Vec<Package> {
    let mut packages = Vec::new();

    if let Some(flatpak) = find_command("flatpak") {
        if let Ok(output) = run_command(
            &flatpak,
            &["list", "--app", "--columns=application,version,description"],
            "Flatpak application list",
        ) {
            for line in output_lines(output) {
                let parts: Vec<&str> = line.split('\t').collect();
                let Some(name) = parts
                    .first()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                else {
                    continue;
                };
                packages.push(package(
                    format!("flatpak:{name}"),
                    name,
                    parts
                        .get(2)
                        .map(|value| value.trim().to_owned())
                        .filter(|value| !value.is_empty()),
                    parts.get(1).map(|value| value.trim()).unwrap_or("Unknown"),
                    true,
                    "flatpak",
                    0,
                ));
            }
        }
    }

    if let Some(snap) = find_command("snap") {
        if let Ok(output) = run_command(&snap, &["list"], "Snap package list") {
            for line in output_lines(output).skip(1) {
                let mut parts = line.split_whitespace();
                let Some(name) = parts.next() else {
                    continue;
                };
                let version = parts.next().unwrap_or("Unknown");
                packages.push(package(
                    format!("snap:{name}"),
                    name,
                    None,
                    version,
                    true,
                    "snap",
                    0,
                ));
            }
        }
    }

    packages
}

#[cfg(target_os = "linux")]
fn get_linux_packages() -> Result<Vec<Package>, String> {
    let manager = PackageManager::detect().ok_or_else(|| {
        "No supported Linux package manager was found (APT, DNF, Pacman, Zypper, Portage, or XBPS)."
            .to_string()
    })?;
    let mut packages = get_primary_linux_packages(manager)?;
    packages.extend(supplemental_linux_packages());
    Ok(packages)
}

#[cfg(target_os = "windows")]
fn get_windows_packages() -> Result<Vec<Package>, String> {
    let packages: Vec<Package> = windows_registry::installed_programs()
        .into_iter()
        .filter(|program| !program.is_system_component)
        .map(|program| {
            package(
                program.id,
                program.display_name,
                program.publisher,
                program
                    .display_version
                    .unwrap_or_else(|| "Unknown".to_string()),
                program.display_icon.is_some(),
                "windows-registry",
                program.estimated_size_bytes.unwrap_or(0),
            )
        })
        .collect();

    if packages.is_empty() {
        return Err(
            "Windows did not report any installed programs in the current user's registry views."
                .to_string(),
        );
    }

    Ok(packages)
}

pub fn get_installed_packages() -> Result<Vec<Package>, String> {
    #[cfg(target_os = "macos")]
    let mut packages = get_macos_packages()?;

    #[cfg(target_os = "linux")]
    let mut packages = get_linux_packages()?;

    #[cfg(target_os = "windows")]
    let mut packages = get_windows_packages()?;

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let mut packages: Vec<Package> = Vec::new();

    packages.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.manager.cmp(&right.manager))
    });
    Ok(packages)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn package_name_from_id(id: &str) -> &str {
    id.split_once(':').map(|(_, name)| name).unwrap_or(id)
}

#[cfg(target_os = "macos")]
fn uninstall_macos_package(id: &str, is_cask: bool) -> Result<(), String> {
    let brew = brew_path()?;
    let name = package_name_from_id(id);
    let mut args = vec!["uninstall"];
    if is_cask {
        args.push("--cask");
    }
    args.push(name);
    run_command(&brew, &args, "Homebrew uninstall").map(|_| ())
}

#[cfg(target_os = "linux")]
fn uninstall_linux_package(id: &str, manager: Option<&str>) -> Result<(), String> {
    let manager = manager
        .or_else(|| id.split_once(':').map(|(manager, _)| manager))
        .unwrap_or_default();
    let name = package_name_from_id(id);

    let (program, args): (&str, Vec<&str>) = match manager {
        "dnf" => ("pkexec", vec!["dnf", "remove", "-y", name]),
        "apt" => ("pkexec", vec!["env", "DEBIAN_FRONTEND=noninteractive", "apt-get", "remove", "-y", name]),
        "pacman" => ("pkexec", vec!["pacman", "-R", "--noconfirm", name]),
        "zypper" => (
            "pkexec",
            vec!["zypper", "--non-interactive", "remove", name],
        ),
        "emerge" => ("pkexec", vec!["emerge", "--deselect", name]),
        "xbps" => ("pkexec", vec!["xbps-remove", "-y", name]),
        "flatpak" => ("flatpak", vec!["uninstall", "--noninteractive", "-y", name]),
        "snap" => ("pkexec", vec!["snap", "remove", name]),
        "homebrew" => ("brew", vec!["uninstall", name]),
        _ => return Err(format!("Unsupported package source: {manager}")),
    };

    let command = find_command(program)
        .ok_or_else(|| format!("Required command `{program}` was not found."))?;
    run_command(&command, &args, "package uninstall").map(|_| ())
}

#[cfg(target_os = "windows")]
fn uninstall_windows_package(id: &str) -> Result<(), String> {
    let program = windows_registry::find_installed_program(id)
        .ok_or_else(|| "The selected program is no longer registered by Windows.".to_string())?;
    let mut uninstall_command = program
        .quiet_uninstall_string
        .or(program.uninstall_string)
        .ok_or_else(|| "This program does not provide an uninstall command.".to_string())?;

    // Registry entries for MSI software commonly use `/I` (maintenance mode).
    // Switch it to `/X` so the confirmed action is actually an uninstall.
    if uninstall_command.to_ascii_lowercase().contains("msiexec") {
        uninstall_command = uninstall_command
            .replace(" /I{", " /X{")
            .replace(" /i{", " /x{")
            .replace(" /I ", " /X ")
            .replace(" /i ", " /x ");
    }

    let cmd = find_command("cmd")
        .ok_or_else(|| "Windows command processor was not found.".to_string())?;
    let output = background_command(&cmd)
        .args(["/D", "/S", "/C", &uninstall_command])
        .output()
        .map_err(|error| format!("Could not launch the Windows uninstaller: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if stderr.is_empty() {
            format!("Windows uninstaller exited with {}", output.status)
        } else {
            format!("Windows uninstaller failed: {stderr}")
        })
    }
}

pub fn uninstall_package(id: &str, is_cask: bool, manager: Option<&str>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = manager;
        return uninstall_macos_package(id, is_cask);
    }

    #[cfg(target_os = "linux")]
    {
        let _ = is_cask;
        return uninstall_linux_package(id, manager);
    }

    #[cfg(target_os = "windows")]
    {
        let _ = (is_cask, manager);
        return uninstall_windows_package(id);
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = (id, is_cask, manager);
        Err("Package management is not supported on this platform.".to_string())
    }
}
