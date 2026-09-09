//! Safety-tier classification.
//!
//! Order of checks (first hit wins):
//! 1. Known-reclaimable rules database (most specific, includes paths under
//!    C:\Windows like the Delivery Optimization cache)
//! 2. Store package folders (WindowsApps, %LOCALAPPDATA%\Packages)
//! 3. OS-critical path prefixes
//! 4. Classic-app install locations from the uninstall registry
//! 5. User content folders (Documents, Pictures, …)
//! 6. AppData folder-name match against installed app names (heuristic)
//!
//! Phase 3 (staleness) will split AppInstalled into AppActive/AppStale using
//! Prefetch/UserAssist/BAM signals.

use std::path::Path;

use serde::Serialize;

use crate::ownership::{
    is_generic_appdata_name, load_installed_apps, package_display_name, InstalledApp,
};
use crate::rules::{RuleAction, RuleSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    /// Under an OS-managed path. Never offer deletion; point at Storage
    /// Sense / DISM instead.
    OsCritical,
    /// Owned by an installed application (Store or classic).
    AppInstalled,
    /// Owned by an app that ran recently. (Phase 3)
    AppActive,
    /// Owned by an app with no recorded use in the staleness window. (Phase 3)
    AppStale,
    /// Matched the reclaimable rules database (caches, temp, shader caches…).
    KnownReclaimable,
    /// Personal files (Documents, Pictures, …) — yours to judge.
    UserContent,
    /// Big and unclassified — surfaced for the human to decide.
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Classification {
    pub tier: Tier,
    /// Which known-reclaimable rule matched, if any.
    pub rule_id: Option<String>,
    /// Owning app's display name, when one was identified.
    pub owner: Option<String>,
    /// One-line human explanation for the UI.
    pub note: Option<String>,
}

impl Classification {
    fn plain(tier: Tier, note: Option<String>) -> Self {
        Self {
            tier,
            rule_id: None,
            owner: None,
            note,
        }
    }
}

const USER_CONTENT_DIRS: [&str; 7] = [
    "documents",
    "pictures",
    "videos",
    "music",
    "downloads",
    "desktop",
    "onedrive",
];

pub struct Classifier {
    rules: RuleSet,
    /// Name-based heuristics, consulted only when nothing precise matched.
    fallback_rules: RuleSet,
    os_prefixes: Vec<String>,
    apps: Vec<InstalledApp>,
}

impl Classifier {
    pub fn new() -> Self {
        let windir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
        Self {
            rules: RuleSet::builtin(),
            fallback_rules: RuleSet::builtin_fallback(),
            os_prefixes: vec![
                normalize(&windir),
                normalize(r"C:\System Volume Information"),
                normalize(r"C:\$Recycle.Bin"),
                normalize(r"C:\ProgramData\Microsoft"),
            ],
            apps: load_installed_apps(),
        }
    }

    pub fn classify(&self, path: &Path) -> Classification {
        let normalized = normalize(&path.display().to_string());
        let segments: Vec<&str> = normalized.split('/').collect();

        if let Some(rule) = self.rules.match_path(&normalized) {
            let action = match rule.action {
                RuleAction::Safe => "Safe to delete",
                RuleAction::Review => "Reclaimable after review",
                RuleAction::SystemTool => "Reclaim via the tool that manages it",
            };
            return Classification {
                tier: Tier::KnownReclaimable,
                rule_id: Some(rule.id.clone()),
                owner: None,
                note: Some(format!("{action}: {}", rule.description)),
            };
        }

        // Store apps: c:/program files/windowsapps/<pkg> and
        // c:/users/<u>/appdata/local/packages/<pkg>
        if let Some(pkg) = store_package_segment(&segments) {
            let name = package_display_name(pkg);
            return Classification {
                tier: Tier::AppInstalled,
                rule_id: None,
                owner: Some(name.clone()),
                note: Some(format!(
                    "Microsoft Store app '{name}' — uninstall from Settings > Apps to reclaim"
                )),
            };
        }

        if self
            .os_prefixes
            .iter()
            .any(|prefix| starts_with_component(&normalized, prefix))
        {
            return Classification::plain(
                Tier::OsCritical,
                Some("Part of Windows — manage with Storage Sense or Disk Cleanup, never delete manually".into()),
            );
        }

        if let Some(app) = self.apps.iter().find(|app| app.owns(&normalized)) {
            return Classification {
                tier: Tier::AppInstalled,
                rule_id: None,
                owner: Some(app.name.clone()),
                note: Some(format!(
                    "Install folder of '{}' — uninstalling the app reclaims it",
                    app.name
                )),
            };
        }

        // c:/users/<name>/<content dir>/...
        if segments.len() >= 3
            && segments.first() == Some(&"c:")
            && segments.get(1) == Some(&"users")
        {
            if let Some(third) = segments.get(3) {
                if USER_CONTENT_DIRS.contains(third) {
                    return Classification::plain(
                        Tier::UserContent,
                        Some("Your personal files — only you can judge these".into()),
                    );
                }
            }
            // c:/users/<name>/appdata/{local,roaming,locallow}/<vendor>/...
            if segments.get(3) == Some(&"appdata") {
                if let Some(vendor) = segments.get(5) {
                    if !is_generic_appdata_name(vendor) {
                        if let Some(app) = self.apps.iter().find(|app| {
                            let app_name = app.name.to_lowercase();
                            app_name.contains(vendor) || vendor.contains(&app_name)
                        }) {
                            return Classification {
                                tier: Tier::AppInstalled,
                                rule_id: None,
                                owner: Some(app.name.clone()),
                                note: Some(format!(
                                    "App data likely belonging to '{}' (name match)",
                                    app.name
                                )),
                            };
                        }
                    }
                }
            }
        }

        // Last resort before Unknown: the folder's own name says cache/temp/log.
        if let Some(rule) = self.fallback_rules.match_path(&normalized) {
            return Classification {
                tier: Tier::KnownReclaimable,
                rule_id: Some(rule.id.clone()),
                owner: None,
                note: Some(format!("Reclaimable after review: {}", rule.description)),
            };
        }

        Classification::plain(Tier::Unknown, None)
    }
}

impl Default for Classifier {
    fn default() -> Self {
        Self::new()
    }
}

fn store_package_segment<'a>(segments: &[&'a str]) -> Option<&'a str> {
    // c:/program files/windowsapps/<pkg>
    if segments.first() == Some(&"c:")
        && segments.get(1) == Some(&"program files")
        && segments.get(2) == Some(&"windowsapps")
    {
        return segments.get(3).copied();
    }
    // c:/users/<u>/appdata/local/packages/<pkg>
    if segments.first() == Some(&"c:")
        && segments.get(1) == Some(&"users")
        && segments.get(3) == Some(&"appdata")
        && segments.get(4) == Some(&"local")
        && segments.get(5) == Some(&"packages")
    {
        return segments.get(6).copied();
    }
    None
}

/// Prefix match that only matches whole path components.
pub(crate) fn starts_with_component(path: &str, prefix: &str) -> bool {
    path == prefix
        || (path.starts_with(prefix) && path.as_bytes().get(prefix.len()) == Some(&b'/'))
}

/// Lowercased, forward-slash form used for all matching.
pub(crate) fn normalize(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_dir_is_os_critical() {
        let c = Classifier::new();
        assert_eq!(
            c.classify(Path::new(r"C:\Windows\System32")).tier,
            Tier::OsCritical
        );
    }

    #[test]
    fn node_modules_is_reclaimable() {
        let c = Classifier::new();
        let result = c.classify(Path::new(r"D:\code\my-app\node_modules"));
        assert_eq!(result.tier, Tier::KnownReclaimable);
        assert_eq!(result.rule_id.as_deref(), Some("node-modules"));
        assert!(result.note.is_some());
    }

    #[test]
    fn store_package_is_app_owned() {
        let c = Classifier::new();
        let result = c.classify(Path::new(
            r"C:\Users\richa\AppData\Local\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe",
        ));
        assert_eq!(result.tier, Tier::AppInstalled);
        assert_eq!(result.owner.as_deref(), Some("microsoft.windowsterminal"));
    }

    #[test]
    fn documents_is_user_content() {
        let c = Classifier::new();
        assert_eq!(
            c.classify(Path::new(r"C:\Users\richa\Documents\taxes")).tier,
            Tier::UserContent
        );
    }

    #[test]
    fn random_dir_is_unknown() {
        let c = Classifier::new();
        assert_eq!(c.classify(Path::new(r"D:\photos\2024")).tier, Tier::Unknown);
    }

    #[test]
    fn cache_named_dir_hits_fallback() {
        let c = Classifier::new();
        let result = c.classify(Path::new(r"D:\tools\some-utility\Cache"));
        assert_eq!(result.tier, Tier::KnownReclaimable);
        assert_eq!(result.rule_id.as_deref(), Some("named-cache"));
    }

    #[test]
    fn fallback_does_not_override_os_or_user_content() {
        let c = Classifier::new();
        // System32 contains no cache-y name; stays OS regardless of fallback.
        assert_eq!(
            c.classify(Path::new(r"C:\Windows\System32\config")).tier,
            Tier::OsCritical
        );
        // A "cache" inside Documents stays the user's business.
        assert_eq!(
            c.classify(Path::new(r"C:\Users\richa\Documents\project\cache")).tier,
            Tier::UserContent
        );
    }

    #[test]
    fn new_rules_match() {
        let c = Classifier::new();
        let dumps = c.classify(Path::new(r"C:\Users\richa\AppData\Local\CrashDumps"));
        assert_eq!(dumps.rule_id.as_deref(), Some("crash-dumps"));
        let wu = c.classify(Path::new(r"C:\Windows\SoftwareDistribution\Download"));
        assert_eq!(wu.rule_id.as_deref(), Some("windows-update-cache"));
        let target = c.classify(Path::new(r"D:\code\my-app\target\debug"));
        assert_eq!(target.rule_id.as_deref(), Some("rust-target"));
    }
}
