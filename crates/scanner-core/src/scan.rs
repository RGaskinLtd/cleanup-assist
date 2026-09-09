//! Phase 1 scanner: parallel directory walk with per-directory size aggregation.
//!
//! Known limitations, addressed in later phases:
//! - Uses logical file size, not size-on-disk. OneDrive cloud-only placeholders
//!   and NTFS-compressed/sparse files are over-counted (needs
//!   `GetCompressedFileSizeW` / `FSCTL_GET_RETRIEVAL_POINTERS`).
//! - No hardlink dedupe yet, so WinSxS appears larger than the space it costs
//!   (needs file-ID dedupe via `BY_HANDLE_FILE_INFORMATION`).
//! - Walks the directory tree via ReadDirectoryW. An elevated NTFS MFT reader
//!   is the planned fast path; this walk stays as the non-elevated fallback.
//!
//! Junctions and symlinks are never traversed (they'd double-count or loop).

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use jwalk::WalkDir;

use crate::classify::Classifier;
use crate::types::{DirStat, FileStat, ScanProgress, ScanResult};

#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// How many directories to report back.
    pub top_dirs: usize,
    /// How many individual files to report back.
    pub top_files: usize,
    /// Skip a directory in the report when one child holds more than this
    /// fraction of its size — the child is the more informative entry.
    pub dominance_threshold: f64,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            top_dirs: 50,
            top_files: 25,
            dominance_threshold: 0.95,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("path does not exist or is not a directory: {0}")]
    RootNotFound(String),
}

pub fn scan(root: &Path, opts: &ScanOptions) -> Result<ScanResult, ScanError> {
    scan_with_progress(root, opts, &mut |_| {})
}

/// Like [`scan`], streaming running counters through `progress` (throttled to
/// a few events per second).
pub fn scan_with_progress(
    root: &Path,
    opts: &ScanOptions,
    progress: &mut dyn FnMut(ScanProgress),
) -> Result<ScanResult, ScanError> {
    if !root.is_dir() {
        return Err(ScanError::RootNotFound(root.display().to_string()));
    }
    let start = Instant::now();
    let mut bytes_seen: u64 = 0;
    let mut last_emit = Instant::now();

    // Direct (non-recursive) file bytes per directory.
    let mut direct: HashMap<PathBuf, u64> = HashMap::new();
    let mut file_count: u64 = 0;
    let mut dir_count: u64 = 0;
    let mut skipped: u64 = 0;
    // Min-heap of the N largest files seen so far.
    let mut largest: BinaryHeap<Reverse<(u64, PathBuf)>> = BinaryHeap::new();

    for entry in WalkDir::new(root).skip_hidden(false).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let file_type = entry.file_type();
        if file_type.is_dir() {
            dir_count += 1;
            continue;
        }
        // Symlinks and junctions report as symlinks and are neither counted
        // nor traversed.
        if !file_type.is_file() {
            continue;
        }
        let bytes = match entry.metadata() {
            Ok(m) => m.len(),
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        file_count += 1;
        bytes_seen += bytes;
        let path = entry.path();
        if let Some(parent) = path.parent() {
            *direct.entry(parent.to_path_buf()).or_default() += bytes;
            if last_emit.elapsed() >= Duration::from_millis(150) {
                progress(ScanProgress {
                    phase: "walking".into(),
                    files: file_count,
                    dirs: dir_count,
                    bytes: bytes_seen,
                    current_path: parent.display().to_string(),
                });
                last_emit = Instant::now();
            }
        }
        largest.push(Reverse((bytes, path)));
        if largest.len() > opts.top_files {
            largest.pop();
        }
    }

    // The roll-up over ~1M directories takes a moment of its own.
    progress(ScanProgress {
        phase: "aggregating".into(),
        files: file_count,
        dirs: dir_count,
        bytes: bytes_seen,
        current_path: String::new(),
    });

    // Roll direct sizes up into cumulative per-directory totals.
    let mut cumulative: HashMap<PathBuf, u64> = HashMap::new();
    for (dir, bytes) in &direct {
        let mut current = Some(dir.as_path());
        while let Some(d) = current {
            *cumulative.entry(d.to_path_buf()).or_default() += bytes;
            if d == root {
                break;
            }
            current = d.parent();
        }
    }
    let total_bytes = cumulative.get(root).copied().unwrap_or(0);

    // Parent -> largest child size, used for the dominance filter below.
    let mut largest_child: HashMap<&Path, u64> = HashMap::new();
    for (dir, bytes) in &cumulative {
        if let Some(parent) = dir.parent() {
            let slot = largest_child.entry(parent).or_default();
            *slot = (*slot).max(*bytes);
        }
    }

    let classifier = Classifier::new();
    let mut ranked: Vec<(&PathBuf, &u64)> = cumulative
        .iter()
        .filter(|(dir, bytes)| {
            if dir.as_path() == root || **bytes == 0 {
                return false;
            }
            // Drop chain links: if one child holds nearly all of this
            // directory's bytes, reporting the child alone is clearer.
            match largest_child.get(dir.as_path()) {
                Some(child_bytes) => {
                    (*child_bytes as f64) < (**bytes as f64) * opts.dominance_threshold
                }
                None => true,
            }
        })
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1));

    let top_dirs: Vec<DirStat> = ranked
        .into_iter()
        .take(opts.top_dirs)
        .map(|(dir, bytes)| {
            let c = classifier.classify(dir);
            DirStat {
                path: dir.display().to_string(),
                bytes: *bytes,
                tier: c.tier,
                rule_id: c.rule_id,
                owner: c.owner,
                note: c.note,
            }
        })
        .collect();

    // Ascending order of Reverse == descending by size, so this is biggest-first.
    let largest_files: Vec<FileStat> = largest
        .into_sorted_vec()
        .into_iter()
        .map(|Reverse((bytes, path))| FileStat {
            path: path.display().to_string(),
            bytes,
        })
        .collect();

    Ok(ScanResult {
        root: root.display().to_string(),
        total_bytes,
        file_count,
        dir_count,
        skipped,
        duration_ms: start.elapsed().as_millis() as u64,
        top_dirs,
        largest_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scans_a_small_tree() {
        let dir = std::env::temp_dir().join("cca-scan-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::write(dir.join("a/one.bin"), vec![0u8; 1000]).unwrap();
        fs::write(dir.join("a/b/two.bin"), vec![0u8; 500]).unwrap();

        let result = scan(&dir, &ScanOptions::default()).unwrap();
        assert_eq!(result.total_bytes, 1500);
        assert_eq!(result.file_count, 2);
        assert_eq!(result.largest_files[0].bytes, 1000);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_root_errors() {
        assert!(scan(Path::new("Z:\\definitely-not-here"), &ScanOptions::default()).is_err());
    }
}
