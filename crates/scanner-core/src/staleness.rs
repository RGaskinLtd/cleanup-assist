//! Phase 3: app usage staleness signals. Stub for now.
//!
//! Windows records app usage in several places; combining them gives a
//! reliable "when did this app last actually run" answer without relying on
//! file access timestamps (which NTFS throttles or disables):
//!
//! | Signal                | Source                                              | Retention        |
//! |-----------------------|-----------------------------------------------------|------------------|
//! | Prefetch              | `C:\Windows\Prefetch\*.pf` (needs admin)            | months           |
//! | UserAssist            | `HKCU\...\Explorer\UserAssist` (ROT13-encoded)      | long-lived       |
//! | BAM                   | `HKLM\SYSTEM\...\Services\bam\State\UserSettings`   | ~1 week          |
//! | Uninstall keys        | `HKLM/HKCU\...\Uninstall` (InstallDate)             | permanent        |
//!
//! Planned deps: `windows-registry` for the registry sources; Prefetch files
//! have a documented binary format (MAM-compressed on Win11).

use std::time::SystemTime;

/// When an application executable was last observed running, and by which
/// signal, so the UI can show provenance ("last ran 2025-11-03 via Prefetch").
#[derive(Debug, Clone)]
pub struct LastUsed {
    pub when: SystemTime,
    pub signal: Signal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Prefetch,
    UserAssist,
    Bam,
}

/// Best-effort "when did this exe last run", newest signal wins.
///
/// TODO Phase 3: implement Prefetch + UserAssist + BAM readers.
pub fn app_last_used(_exe_path: &str) -> Option<LastUsed> {
    None
}
