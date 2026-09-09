use serde::Serialize;

use crate::classify::Tier;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub root: String,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    /// Entries we could not read (permissions, in-use files). Common on C:\.
    pub skipped: u64,
    pub duration_ms: u64,
    pub top_dirs: Vec<DirStat>,
    pub largest_files: Vec<FileStat>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirStat {
    pub path: String,
    pub bytes: u64,
    pub tier: Tier,
    /// Which known-reclaimable rule matched, if any (id from the rules database).
    pub rule_id: Option<String>,
    /// Owning app's display name, when identified.
    pub owner: Option<String>,
    /// One-line human explanation of the classification.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    pub path: String,
    pub bytes: u64,
}

/// Progress event streamed to the UI while a scan runs. Totals aren't known
/// until the walk finishes, so these are running counters, not a percentage.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    /// "walking" (discovering files) or "aggregating" (rolling up sizes).
    pub phase: String,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub current_path: String,
}
