//! Core engine for Cleanup Chore Assist.
//!
//! Pipeline: `scan` walks the filesystem and aggregates sizes, `classify`
//! assigns each large directory a safety tier, `staleness` (Phase 3) will
//! date-stamp when the owning application was last actually used.

pub mod classify;
pub mod ownership;
pub mod relocate;
pub mod rules;
pub mod scan;
pub mod staleness;
pub mod types;

pub use classify::{Classification, Classifier, Tier};
pub use relocate::{
    relocate_dir, relocate_dir_with_progress, RelocateError, RelocateProgress, RelocateReport,
};
pub use scan::{scan, scan_with_progress, ScanError, ScanOptions};
pub use types::{DirStat, FileStat, ScanProgress, ScanResult};
