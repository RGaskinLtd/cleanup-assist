# Cleanup Chore Assist

A Windows 11 disk-space analyzer that doesn't stop at "this folder is big" — it tells you
**whether the space is safe to take back**, by classifying what it finds against OS-critical
paths, a known-reclaimable rules database, and (eventually) real app-usage history.

Status: **Phase 1 scaffold.** Scanning and basic classification work; see
[docs/SPEC.md](docs/SPEC.md) for the full plan.

## Run it

Prerequisites: Node 20+, Rust (stable, MSVC toolchain), WebView2 (preinstalled on Win11).

```
npm install
npm run tauri dev
```

`npm run tauri build` produces an installer under `target/release/bundle`.

## Architecture

```
crates/scanner-core/     The engine (pure Rust, no Tauri dependency)
  src/scan.rs            Parallel directory walk + size aggregation
  src/classify.rs        Safety tiers (os-critical / app-installed / reclaimable …)
  src/ownership.rs       Uninstall-registry + Store-package app ownership
  src/relocate.rs        Move dir to another drive + leave a junction
  src/rules.rs           Known-reclaimable rules engine (glob-based)
  rules/reclaimable.json Built-in rules database
  src/staleness.rs       App last-used signals (Phase 3 stub)
src-tauri/               Tauri shell: scan_path / list_drives / relocate_dir
src/                     React UI (drive picker, tier badges, legend, Move…)
```

Tests: `cargo test -p scanner-core`
