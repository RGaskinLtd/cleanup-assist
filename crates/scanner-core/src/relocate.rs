//! Move a directory to another drive and leave an NTFS junction at the old
//! path, so apps that reference the old location keep working.
//!
//! Junctions (not symlinks) on purpose: creating them needs no admin rights
//! and every Windows API resolves them transparently.
//!
//! Ordering is chosen so the user's data is never the only copy mid-failure,
//! and so the step that fails when files are in use (the rename) happens
//! FIRST — before gigabytes get copied, and freezing the tree so the copy
//! can't race concurrent writes through the original path:
//! 1. rename the original to `<name>.cca-backup` (same volume, atomic;
//!    fails fast with FolderInUse if anything inside is open)
//! 2. copy the backup to the destination
//! 3. create the junction at the original path -> destination
//! 4. verify the junction resolves
//! 5. only then delete the backup
//! Any failure before step 5 rolls back to the original layout.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::classify::{Classifier, Tier};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelocateReport {
    pub src: String,
    pub dest: String,
    pub bytes_moved: u64,
    pub files_moved: u64,
    /// Reparse points (junctions/symlinks) inside the source are not copied.
    pub links_skipped: u64,
    /// Set when the backup could not be fully deleted (locked files); the
    /// move succeeded but this path must be removed manually to free space.
    pub backup_left_at: Option<String>,
}

/// Progress event streamed to the UI while a move runs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelocateProgress {
    /// "preparing" (sizing the tree), "copying", or "finalizing"
    /// (junction + cleanup).
    pub phase: String,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub current_file: String,
}

struct ProgressSink<'a> {
    bytes_total: u64,
    /// Bytes of fully copied files; the in-flight file's progress rides on top.
    bytes_done: u64,
    current_file: String,
    last_emit: Instant,
    emit: &'a mut dyn FnMut(RelocateProgress),
}

impl ProgressSink<'_> {
    fn emit_now(&mut self, phase: &str, in_file_bytes: u64) {
        (self.emit)(RelocateProgress {
            phase: phase.to_string(),
            bytes_done: self.bytes_done + in_file_bytes,
            bytes_total: self.bytes_total,
            current_file: self.current_file.clone(),
        });
        self.last_emit = Instant::now();
    }

    /// Rate-limited to ~10 events/sec; called from the per-chunk copy callback.
    fn emit_throttled(&mut self, in_file_bytes: u64) {
        if self.last_emit.elapsed() >= Duration::from_millis(100) {
            self.emit_now("copying", in_file_bytes);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RelocateError {
    #[error("source does not exist or is not a directory: {0}")]
    SourceNotFound(String),
    #[error("refusing to move an OS-managed folder: {0}")]
    SourceIsOsCritical(String),
    #[error("refusing to move a drive root")]
    SourceIsRoot,
    #[error("source is already a junction or symlink: {0}")]
    SourceIsLink(String),
    #[error("destination folder does not exist: {0}")]
    DestNotFound(String),
    #[error("destination must not be inside the source")]
    DestInsideSource,
    #[error("{0} already exists — pick a different destination")]
    DestOccupied(String),
    #[error("not enough free space at destination: need {needed} bytes, have {available}")]
    InsufficientSpace { needed: u64, available: u64 },
    #[error("copy failed at {path}: {source}")]
    CopyFailed {
        path: String,
        source: std::io::Error,
    },
    #[error("this folder contains Cleanup Chore Assist's own {0} — the app can't move the folder it is running from")]
    SourceContainsSelf(&'static str),
    #[error("files in this folder are in use — close programs using it (and any Explorer windows open on it), then try again. Windows reported: {0}")]
    FolderInUse(std::io::Error),
    #[error("could not set aside the original folder: {0}")]
    BackupRenameFailed(std::io::Error),
    #[error("a previous move left {0} behind — restore or delete it before moving this folder")]
    BackupLeftover(String),
    #[error("junction creation failed (original restored): {0}")]
    JunctionFailed(std::io::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn relocate_dir(src: &Path, dest_parent: &Path) -> Result<RelocateReport, RelocateError> {
    relocate_dir_with_progress(src, dest_parent, &mut |_| {})
}

/// Like [`relocate_dir`], streaming progress through `progress` (throttled to
/// ~10 events/sec while copying).
pub fn relocate_dir_with_progress(
    src: &Path,
    dest_parent: &Path,
    progress: &mut dyn FnMut(RelocateProgress),
) -> Result<RelocateReport, RelocateError> {
    let src_meta = fs::symlink_metadata(src)
        .map_err(|_| RelocateError::SourceNotFound(src.display().to_string()))?;
    if src_meta.file_type().is_symlink() {
        return Err(RelocateError::SourceIsLink(src.display().to_string()));
    }
    if !src_meta.is_dir() {
        return Err(RelocateError::SourceNotFound(src.display().to_string()));
    }
    let Some(dir_name) = src.file_name() else {
        return Err(RelocateError::SourceIsRoot);
    };
    let tier = Classifier::new().classify(src).tier;
    if tier == Tier::OsCritical {
        return Err(RelocateError::SourceIsOsCritical(src.display().to_string()));
    }
    if let Some(what) = contains_self(src) {
        return Err(RelocateError::SourceContainsSelf(what));
    }
    if !dest_parent.is_dir() {
        return Err(RelocateError::DestNotFound(dest_parent.display().to_string()));
    }
    if dest_parent.starts_with(src) {
        return Err(RelocateError::DestInsideSource);
    }
    let dest = dest_parent.join(dir_name);
    if dest.exists() {
        return Err(RelocateError::DestOccupied(dest.display().to_string()));
    }

    progress(RelocateProgress {
        phase: "preparing".into(),
        bytes_done: 0,
        bytes_total: 0,
        current_file: String::new(),
    });
    let needed = dir_size(src)?;
    let available = fs2::available_space(dest_parent)?;
    // 5% headroom for allocation overhead on the destination volume.
    if needed + needed / 20 > available {
        return Err(RelocateError::InsufficientSpace { needed, available });
    }

    // Append rather than with_extension(): "my.folder" must not become
    // "my.cca-backup".
    let mut backup_name = dir_name.to_os_string();
    backup_name.push(".cca-backup");
    let backup = src.with_file_name(backup_name);
    if backup.exists() {
        return Err(RelocateError::BackupLeftover(backup.display().to_string()));
    }

    // Set the original aside BEFORE copying: this rename is what Windows
    // refuses (os error 5) when anything inside is open, so it must fail
    // fast, not after a long copy. Retries absorb transient locks from
    // antivirus scans and indexing.
    if let Err(e) = rename_with_retry(src, &backup) {
        return Err(match e.raw_os_error() {
            // 5 = ERROR_ACCESS_DENIED (open handles below), 32 = sharing violation
            Some(5) | Some(32) => RelocateError::FolderInUse(e),
            _ => RelocateError::BackupRenameFailed(e),
        });
    }

    let mut report = RelocateReport {
        src: src.display().to_string(),
        dest: dest.display().to_string(),
        bytes_moved: 0,
        files_moved: 0,
        links_skipped: 0,
        backup_left_at: None,
    };
    let mut sink = ProgressSink {
        bytes_total: needed,
        bytes_done: 0,
        current_file: String::new(),
        last_emit: Instant::now(),
        emit: progress,
    };
    sink.emit_now("copying", 0);
    if let Err(e) = copy_tree(&backup, &dest, &mut report, &mut sink) {
        let _ = fs::remove_dir_all(&dest); // clean up the partial copy
        let _ = rename_with_retry(&backup, src); // restore the original
        return Err(e);
    }
    sink.emit_now("finalizing", 0);

    if let Err(e) = junction::create(&dest, src).and_then(|()| fs::read_dir(src).map(|_| ())) {
        // Roll back: drop the half-made junction, restore the original.
        let _ = fs::remove_dir_all(src);
        let _ = rename_with_retry(&backup, src);
        let _ = fs::remove_dir_all(&dest);
        return Err(RelocateError::JunctionFailed(e));
    }

    if fs::remove_dir_all(&backup).is_err() {
        report.backup_left_at = Some(backup.display().to_string());
    }
    Ok(report)
}

/// The process holds handles that make moving these folders impossible: the
/// running executable's folder and the current working directory. Detect them
/// up front so the error says "that's me" instead of "close the program".
fn contains_self(src: &Path) -> Option<&'static str> {
    // Canonicalize both sides so \\?\-prefixed, short-name (8.3), and
    // junction-resolved forms compare equal; fall back to the raw path.
    let canon = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let src_norm = crate::classify::normalize(&canon(src).display().to_string());
    // Matches the `identifier` in tauri.conf.json — where WebView2 keeps this
    // app's own browser-engine data while it runs.
    let webview_data = std::env::var("LOCALAPPDATA")
        .ok()
        .map(|d| Path::new(&d).join("com.richardgaskin.cleanupchoreassist"));
    let candidates: [(&'static str, Option<std::path::PathBuf>); 3] = [
        ("executable", std::env::current_exe().ok()),
        ("working directory", std::env::current_dir().ok()),
        ("data folder", webview_data),
    ];
    for (what, path) in candidates {
        let Some(path) = path else { continue };
        let path_norm = crate::classify::normalize(&canon(&path).display().to_string());
        if crate::classify::starts_with_component(&path_norm, &src_norm) {
            return Some(what);
        }
    }
    None
}

fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut last_err = None;
    for attempt in 0..4u64 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(150 * attempt));
        }
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap())
}

fn dir_size(dir: &Path) -> std::io::Result<u64> {
    let mut total = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let meta = fs::symlink_metadata(entry.path())?;
        if meta.file_type().is_symlink() {
            continue;
        } else if meta.is_dir() {
            total += dir_size(&entry.path())?;
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}

fn copy_tree(
    src: &Path,
    dest: &Path,
    report: &mut RelocateReport,
    sink: &mut ProgressSink<'_>,
) -> Result<(), RelocateError> {
    fs::create_dir_all(dest).map_err(|e| RelocateError::CopyFailed {
        path: dest.display().to_string(),
        source: e,
    })?;
    let entries = fs::read_dir(src).map_err(|e| RelocateError::CopyFailed {
        path: src.display().to_string(),
        source: e,
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| RelocateError::CopyFailed {
            path: src.display().to_string(),
            source: e,
        })?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let meta = fs::symlink_metadata(&from).map_err(|e| RelocateError::CopyFailed {
            path: from.display().to_string(),
            source: e,
        })?;
        if meta.file_type().is_symlink() {
            // Junctions/symlinks inside the tree are not recreated; copying
            // through them would duplicate their targets (or loop).
            report.links_skipped += 1;
        } else if meta.is_dir() {
            copy_tree(&from, &to, report, sink)?;
        } else {
            copy_file_with_progress(&from, &to, sink).map_err(|e| RelocateError::CopyFailed {
                path: from.display().to_string(),
                source: e,
            })?;
            report.files_moved += 1;
            report.bytes_moved += meta.len();
            sink.bytes_done += meta.len();
            sink.emit_throttled(0);
        }
    }
    Ok(())
}

/// `fs::copy` replacement using `CopyFileExW`, which reports per-chunk
/// progress mid-file (a lone 60GB vhdx still gets a moving bar) while
/// preserving attributes and timestamps like `CopyFile` does.
fn copy_file_with_progress(
    from: &Path,
    to: &Path,
    sink: &mut ProgressSink<'_>,
) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::CopyFileExW;

    unsafe extern "system" fn routine(
        _total_file_size: i64,
        total_bytes_transferred: i64,
        _stream_size: i64,
        _stream_bytes: i64,
        _stream_number: u32,
        _reason: u32,
        _h_source: windows_sys::Win32::Foundation::HANDLE,
        _h_dest: windows_sys::Win32::Foundation::HANDLE,
        data: *const core::ffi::c_void,
    ) -> u32 {
        let sink = unsafe { &mut *(data as *mut ProgressSink) };
        sink.emit_throttled(total_bytes_transferred as u64);
        0 // PROGRESS_CONTINUE
    }

    let wide = |p: &Path| {
        p.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
    };
    sink.current_file = from.display().to_string();
    let ok = unsafe {
        CopyFileExW(
            wide(from).as_ptr(),
            wide(to).as_ptr(),
            Some(routine),
            sink as *mut ProgressSink<'_> as *const core::ffi::c_void,
            std::ptr::null_mut(),
            0,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn relocates_and_links() {
        let root = temp_root("cca-reloc-test");
        let src = root.join("data");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("a.bin"), vec![1u8; 300]).unwrap();
        fs::write(src.join("sub/b.bin"), vec![2u8; 200]).unwrap();
        let dest_parent = root.join("other-drive");
        fs::create_dir_all(&dest_parent).unwrap();

        let report = relocate_dir(&src, &dest_parent).unwrap();
        assert_eq!(report.files_moved, 2);
        assert_eq!(report.bytes_moved, 500);
        assert!(report.backup_left_at.is_none());

        // Old path still works, through the junction…
        assert_eq!(fs::read(src.join("sub/b.bin")).unwrap(), vec![2u8; 200]);
        // …and is actually a reparse point now.
        assert!(fs::symlink_metadata(&src).unwrap().file_type().is_symlink());
        // Real data lives at the destination.
        assert!(dest_parent.join("data/a.bin").is_file());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn in_use_folder_fails_cleanly_before_copying() {
        use std::os::windows::fs::OpenOptionsExt;

        let root = temp_root("cca-reloc-locked");
        let src = root.join("data");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("a.bin"), vec![0u8; 10]).unwrap();
        let dest_parent = root.join("dest");
        fs::create_dir_all(&dest_parent).unwrap();

        // Exclusive handle (share_mode 0) — like an app holding the file open.
        let handle = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(src.join("a.bin"))
            .unwrap();

        let err = relocate_dir(&src, &dest_parent).unwrap_err();
        assert!(
            matches!(err, RelocateError::FolderInUse(_)),
            "unexpected error: {err}"
        );
        // Original is untouched and nothing was copied.
        assert!(src.join("a.bin").is_file());
        assert!(!dest_parent.join("data").exists());

        drop(handle);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reports_progress() {
        let root = temp_root("cca-reloc-progress");
        let src = root.join("data");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("a.bin"), vec![1u8; 4096]).unwrap();
        fs::write(src.join("b.bin"), vec![2u8; 4096]).unwrap();
        let dest_parent = root.join("dest");
        fs::create_dir_all(&dest_parent).unwrap();

        let mut events: Vec<RelocateProgress> = Vec::new();
        let report =
            relocate_dir_with_progress(&src, &dest_parent, &mut |p| events.push(p)).unwrap();
        assert_eq!(report.bytes_moved, 8192);
        assert!(events.iter().any(|e| e.phase == "preparing"));
        assert!(events.iter().any(|e| e.phase == "copying"));
        let done = events.iter().rfind(|e| e.phase == "finalizing").unwrap();
        assert_eq!(done.bytes_done, 8192);
        assert_eq!(done.bytes_total, 8192);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn refuses_to_move_its_own_folder() {
        let exe_dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
        let dest_parent = temp_root("cca-reloc-self");
        let err = relocate_dir(&exe_dir, &dest_parent).unwrap_err();
        assert!(
            matches!(err, RelocateError::SourceContainsSelf(_)),
            "unexpected error: {err}"
        );
        assert!(exe_dir.is_dir());
        let _ = fs::remove_dir_all(&dest_parent);
    }

    #[test]
    fn refuses_dest_inside_source() {
        let root = temp_root("cca-reloc-inside");
        let src = root.join("data");
        fs::create_dir_all(src.join("inner")).unwrap();
        let err = relocate_dir(&src, &src.join("inner")).unwrap_err();
        assert!(matches!(err, RelocateError::DestInsideSource));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn refuses_occupied_destination() {
        let root = temp_root("cca-reloc-occupied");
        let src = root.join("data");
        fs::create_dir_all(&src).unwrap();
        let dest_parent = root.join("dest");
        fs::create_dir_all(dest_parent.join("data")).unwrap();
        let err = relocate_dir(&src, &dest_parent).unwrap_err();
        assert!(matches!(err, RelocateError::DestOccupied(_)));
        let _ = fs::remove_dir_all(&root);
    }
}
