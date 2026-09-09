# Cleanup Chore Assist — Spec

## Problem

Disk analyzers (WinDirStat, WizTree) show *where* the big stuff is, then abandon the user at
the hard part: "is this 30 GB safe to delete?" This app answers size + safety + staleness in
one view, for Windows 11.

## Classification tiers

| Tier | Meaning | UI treatment |
|---|---|---|
| `os-critical` | OS-managed path (`C:\Windows`, WindowsApps, …) | De-emphasized; deep-link Storage Sense / DISM |
| `app-active` | Owned by an app used recently | Leave alone |
| `app-stale` | Owned by an app not run in N months | Flag for review |
| `known-reclaimable` | Matched rules DB (caches, temp, shader caches) | One-click reclaim (recycle, never hard-delete) |
| `unknown` | Big and unclassified | Surface for the human |

## Usage-staleness signals (Phase 3)

File access timestamps are unreliable on NTFS; app-level signals instead:

| Signal | What it gives | Retention |
|---|---|---|
| Prefetch (`C:\Windows\Prefetch`, admin) | Last ~8 run times per exe | months |
| UserAssist (HKCU, ROT13) | Launch counts + last run, GUI apps | long-lived |
| BAM (HKLM registry) | Last execution per user | ~1 week |
| Uninstall keys | InstallLocation, InstallDate | permanent |
| MSIX package API | Exact Store-app file ownership | permanent |
| SRUM (`SRUDB.dat`, ESE format) | Per-app resource history — deferred, painful to parse | 30–60 days |

## Ownership mapping (Phase 2)

1. Store apps: package API — exact (`WindowsApps` + `%LOCALAPPDATA%\Packages`).
2. Classic apps: uninstall registry keys → `InstallLocation`.
3. AppData/ProgramData leftovers: vendor/app name heuristics + rules DB.
4. Anything under `C:\Windows`: OS-critical, full stop.

## Correctness traps (tracked in scan.rs header)

- **Hardlinks**: WinSxS double-counts without file-ID dedupe.
- **OneDrive placeholders**: logical size ≠ size-on-disk (cloud-only files ≈ 0 bytes local).
- **Junctions/symlinks**: never traverse (loops, double-counting). Done.
- **Compressed/sparse files**: need `GetCompressedFileSizeW`.
- **Locked/ACL'd paths**: count as `skipped`, never abort the scan. Done.

## Phases

1. **Scan + report** *(done, walk-based)* — parallel walk, top dirs/files, dominance
   filter, tier badges. Next: MFT fast path (admin), progress events, treemap view.
2. **Classification** *(done, first pass)* — uninstall-registry install locations,
   Store package folders, user-content dirs, AppData name heuristics. Next: better
   AppData attribution, `ProgramData` vendors.
3. **Staleness** — Prefetch/UserAssist/BAM readers; stale apps light up their footprint;
   splits `app-installed` into active/stale.
4. **Actions** *(relocate done)* — move-to-drive + junction (backup kept until the
   junction verifies; junctions need no admin). Next: recycle-bin delete, open in
   Explorer, system-tool deep links, rules-DB user overlay.

## Non-goals

- Auto-deleting anything without explicit user action.
- Touching anything under OS-managed paths directly.
- macOS/Linux (revisit after Windows is solid; scanner-core stays portable-ish).
