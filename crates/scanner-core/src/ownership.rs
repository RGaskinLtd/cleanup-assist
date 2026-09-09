//! Phase 2: map paths to the installed application that owns them.
//!
//! Sources, most to least reliable:
//! 1. MSIX/Store package folders — the folder name *is* the package identity.
//! 2. Classic-app uninstall registry keys → `InstallLocation` prefix match.
//! 3. AppData vendor/app folder-name heuristics against installed app names.

use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub name: String,
    /// Normalized (lowercase, forward-slash) install location, no trailing slash.
    pub location: String,
}

const UNINSTALL_ROOTS: [(winreg::HKEY, &str); 3] = [
    (
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    ),
    (
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ),
    (
        HKEY_CURRENT_USER,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    ),
];

/// Classic (non-Store) apps from the uninstall registry keys. Entries without
/// an `InstallLocation` can't help with path matching and are dropped.
pub fn load_installed_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    for (hive, root) in UNINSTALL_ROOTS {
        let Ok(key) = RegKey::predef(hive).open_subkey(root) else {
            continue;
        };
        for subkey_name in key.enum_keys().flatten() {
            let Ok(entry) = key.open_subkey(&subkey_name) else {
                continue;
            };
            let Ok(name) = entry.get_value::<String, _>("DisplayName") else {
                continue;
            };
            let location = entry
                .get_value::<String, _>("InstallLocation")
                .unwrap_or_default();
            let location = location.trim().trim_end_matches(['\\', '/']);
            if location.is_empty() {
                continue;
            }
            apps.push(InstalledApp {
                name,
                location: crate::classify::normalize(location),
            });
        }
    }
    // Longest location first so the most specific prefix wins a match.
    apps.sort_by(|a, b| b.location.len().cmp(&a.location.len()));
    apps.dedup_by(|a, b| a.location == b.location);
    apps
}

impl InstalledApp {
    pub fn owns(&self, normalized_path: &str) -> bool {
        normalized_path == self.location
            || (normalized_path.starts_with(&self.location)
                && normalized_path.as_bytes().get(self.location.len()) == Some(&b'/'))
    }
}

/// Extract a Store package's friendly name from its folder name, e.g.
/// `microsoft.windowsterminal_1.18_x64__8wekyb3d8bbwe` -> `microsoft.windowsterminal`.
pub fn package_display_name(folder_name: &str) -> String {
    folder_name
        .split('_')
        .next()
        .unwrap_or(folder_name)
        .to_string()
}

/// Folder names under AppData too generic to attribute to an app by name.
pub fn is_generic_appdata_name(segment: &str) -> bool {
    matches!(
        segment,
        "microsoft" | "packages" | "programs" | "temp" | "cache" | "roaming" | "local"
    ) || segment.len() < 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_ownership_requires_separator() {
        let app = InstalledApp {
            name: "Blender".into(),
            location: "c:/program files/blender foundation/blender 4.1".into(),
        };
        assert!(app.owns("c:/program files/blender foundation/blender 4.1/datafiles"));
        assert!(!app.owns("c:/program files/blender foundation/blender 4.10"));
    }

    #[test]
    fn package_name_parses() {
        assert_eq!(
            package_display_name("microsoft.windowsterminal_1.18_x64__8wekyb3d8bbwe"),
            "microsoft.windowsterminal"
        );
    }
}
