# Microsoft Store listing — Cleanup Assist

Copy-paste source for Partner Center → *Store listings*. Field names below match
Partner Center; the limit for each is noted in brackets. Character counts are
current as of writing — re-check with `docs/check-store-limits.py` after edits.

---

## Product name [256]

```
Cleanup Assist
```

---

## Short description [1,000]

```
Every disk analyzer tells you which folders are big. Cleanup Assist tells you which ones are safe to remove.

Scan any drive and every large folder gets a verdict: part of Windows, owned by an installed app, a reclaimable cache, or your own personal files. A built-in database of 46 known space hogs — Docker and WSL virtual disks, node_modules, shader caches, Windows.old, package manager caches, forgotten device backups — explains exactly how to reclaim each one safely.

Short on space but not sure what you can delete? Move a folder to another drive instead. Cleanup Assist copies it across and leaves a junction behind, so every app that used the old location carries on working as if nothing changed.

No accounts, no ads, no telemetry. Nothing is ever deleted or moved unless you click it.
```

---

## Description [10,000]

```
You know the folder is 30 GB. The question every disk analyzer leaves you with is whether deleting it frees your drive or breaks your PC.

Cleanup Assist answers that question. It scans your drive like any other analyzer, then goes one step further: it works out what each large folder actually is, and tells you whether it is safe to reclaim.

WHAT THE BADGES MEAN

Every folder in your results carries one of five verdicts:

• Windows system — part of the operating system. Off limits, and the app tells you to use Storage Sense or Disk Cleanup instead.
• Installed app — matched to a real application through the Windows uninstall registry or Microsoft Store packages, so a cryptic path reads as "Installed app · Blender".
• Reclaimable — matched a known rule, with plain-English guidance on the safe way to clear it.
• Your files — Documents, Pictures, Downloads. Yours to judge; never suggested for cleanup.
• Unclassified — nothing claimed it. Because the first four categories are thorough, this label genuinely means "worth investigating".

THE RULES DATABASE

Cleanup Assist ships with 46 rules covering the folders that quietly eat the most space on a Windows machine: Docker Desktop and WSL virtual disks that grow but never shrink, node_modules directories, GPU shader caches, Windows.old after a feature update, npm, pip, cargo, Gradle, Maven, NuGet and conda caches, crash dumps, browser caches, Visual Studio's installer cache, and device backups people forget they made.

Each rule does more than flag the folder. It tells you whether the contents are safe to delete outright, worth reviewing first, or best handled by the built-in Windows tool that manages them — because deleting some of these by hand causes more problems than it solves.

MOVE INSTEAD OF DELETING

Some folders are too big to keep on your system drive but too useful to delete. Cleanup Assist can move any folder to a roomier drive in one click and leave an NTFS junction in its place, so every application that referenced the old path keeps working exactly as before.

The move is built to be careful with your data. The original folder is set aside first and kept until both the copy and the replacement junction have been verified — if anything fails at any point, your original layout is restored. Free space is checked before a single byte is copied, and folders currently in use are detected up front so a move fails immediately with a fixable message rather than half-finishing.

Progress is byte-accurate and works within a single file, so relocating one 60 GB virtual disk shows a bar that actually moves.

BUILT TO BE TRUSTWORTHY

Cleanup Assist contains no network code whatsoever. No telemetry, no analytics, no crash reporting, no update checks, no accounts. Everything it learns about your disk stays on your machine and is discarded when you close it.

It never acts on its own. Every deletion and every move is something you explicitly start, after reading what the app tells you about that folder.

The full source code is published and readable.

REQUIREMENTS

Windows 11 on a 64-bit Intel or AMD processor. Most features work without administrator rights; running elevated lets the app see folders your account cannot normally read.
```

---

## Product features [up to 20 items, 200 chars each]

```
Scan any drive or folder and see exactly where your disk space went, ranked largest first.
```
```
Live progress while scanning, with running counts of files, folders and bytes so a long scan never looks frozen.
```
```
Every large folder gets a safety verdict: Windows system, installed app, reclaimable cache, your files, or unclassified.
```
```
Identifies the app that owns a folder via the Windows uninstall registry, so cryptic paths read as "Installed app · Blender".
```
```
Detects Microsoft Store app folders and resolves them to the application they belong to.
```
```
Built-in database of 46 known space hogs: Docker and WSL disks, node_modules, shader caches, Windows.old, package caches and more.
```
```
Tells you how to reclaim each folder safely — delete now, review first, or use the Windows tool that manages it.
```
```
Flags unknown cache, temp and log folders for review even when no specific rule matches them.
```
```
Marks your personal files — Documents, Pictures, Downloads — so they are never suggested for cleanup.
```
```
Filter results by safety badge, with a live count for every category.
```
```
Lists the largest individual files separately, for when one enormous file is the real problem.
```
```
Skips chain folders where a single subfolder holds nearly everything, showing the folder that actually matters.
```
```
Unreadable or locked folders are counted as skipped rather than stopping the scan.
```
```
Move any folder to another drive in one click, leaving an NTFS junction so every app keeps working.
```
```
Byte-accurate move progress, including within a single huge file, so a 60 GB virtual disk shows a moving bar.
```
```
Moves roll back cleanly — the original is kept until both the copy and the junction have been verified.
```
```
Checks free space and detects folders in use before starting, so a move fails fast instead of half-finishing.
```
```
Windows system folders are off limits — the app points you to Storage Sense and Disk Cleanup instead.
```
```
Completely offline with zero telemetry. There is no network code in the application at all.
```
```
Nothing is ever deleted or moved without you clicking it. No accounts, no ads, no subscriptions.
```

---

## Search terms [up to 7 terms, 30 chars each]

```
disk space analyzer
disk cleanup
free up disk space
storage analyzer
folder size
disk usage
clean up drive
```

---

## What's new in this version [1,500]

```
Cleanup Assist 0.2.3

• Safety badges now identify the application that owns a folder, using the Windows uninstall registry and Microsoft Store package data.
• The reclaimable rules database has grown to 46 rules, covering Docker and WSL virtual disks, developer caches, shader caches, crash dumps, browser caches and device backups.
• Folders named like caches, temp directories or logs are flagged for review even when no specific rule matches.
• Filter results by safety badge, with a live count per category.
• Live scan progress showing files, folders and bytes as they are found.
• Move a folder to another drive and leave an NTFS junction behind, with byte-accurate progress and full rollback if anything fails.
• Browse to a folder with the file picker as well as typing a path.
```

---

## Additional system requirements [200 per line]

Partner Center splits these into *Minimum* and *Recommended* hardware columns.
Paste one line per requirement.

### Minimum

```
Windows 11 (any edition), 64-bit
```
```
64-bit x64 processor, 1 GHz or faster, 2 or more cores
```
```
4 GB RAM
```
```
60 MB free disk space for installation
```
```
1280 x 720 display
```
```
Microsoft Edge WebView2 Runtime (included with Windows 11)
```

### Recommended

```
Windows 11 22H2 or later, 64-bit
```
```
64-bit x64 processor with 4 or more cores — scanning runs in parallel
```
```
8 GB RAM — a full drive scan of roughly a million files peaks near 500 MB
```
```
Solid-state drive — scan speed is limited by how fast the drive returns file metadata
```
```
Administrator rights, to scan folders a standard account cannot read
```

### Measured on the development machine

SATA SSD, 12 logical cores, Windows 11:

| Scanned | Files | Folders | Time | Peak RAM |
| --- | ---: | ---: | ---: | ---: |
| `C:\Program Files` | 38,124 | 3,491 | 5.0 s | 18 MB |
| A full user profile | 777,914 | 137,210 | 114 s | 293 MB |

Memory tracks the number of folders, not the bytes on disk. Scans on a mechanical
hard drive take considerably longer; the app stays responsive throughout either way.

---

## Copyright and trademark info [200]

```
© 2026 Richard Gaskin. Licensed under PolyForm Internal Use 1.0.0. Not affiliated with Microsoft Corporation.
```

---

## Applicable license terms [10,000]

Point this at the deployed Terms of Use page, or paste the contents of
`site/terms.html` as plain text.

```
https://<your-domain>/terms
```

---

## URLs

| Field | Value |
| --- | --- |
| Privacy policy URL (**required**) | `https://<your-domain>/privacy` |
| Website | `https://<your-domain>` |
| Support contact info | `https://github.com/RGaskinLtd/cleanup-assist/issues` |

---

## Screenshots [1366×768 min, up to 10]

`public/screenshot.png` is 1919×1028 and meets the minimum. Worth adding before
submission: a shot with the badge filter active, and one mid-move showing the
progress bar.

---

## Store artwork

| Asset | File | Size |
| --- | --- | --- |
| Store logo (1:1) | `store-assets/store-logo-2160x2160.png` | 2160×2160 |
| Poster art (2:3) | `store-assets/poster-art-1440x2160.png` | 1440×2160 |

Both are generated from HTML sources in `store-assets/src/`, rendered headlessly
at exact pixel dimensions — edit the source and re-render rather than retouching
the PNGs. The logo is deliberately icon-only and inset from the edges, since the
Store may apply its own corner masking.

---

## Installer parameters

Partner Center asks for the switches that make the package install without any
user interaction. Which value you paste depends on which installer you submit.

### If submitting the NSIS installer (`*-setup.exe`) — recommended

```
/S
```

Verified end to end on `Cleanup Assist_x64-setup.exe`: `/S` installs with no UI
and exits with code 0. The switch is **case-sensitive — uppercase `S`**;
lowercase `/s` is ignored and the installer opens its window instead.

Optional extras, not needed for Store submission:

| Switch | Effect |
| --- | --- |
| `/D=<path>` | Install directory. Must be the **last** argument and **unquoted**, even when the path contains spaces. |
| `/S /D=<path>` | Both together — the order shown is required. |

Uninstall is silent with the same switch: `"<install dir>\uninstall.exe" /S`
removes every file and the registry entry, exit code 0.

### If submitting the MSI

```
/quiet /norestart
```

Partner Center invokes `msiexec /i` itself, so supply only the switches above.
`/qn` is equivalent to `/quiet`. Silent uninstall is `msiexec /x <product> /quiet`.

### Install scope differs between the two

| | NSIS `-setup.exe` | MSI |
| --- | --- | --- |
| Scope | Per-user | Per-machine (`ALLUSERS=1`) |
| Elevation | **Not required** | **Required** |
| Registers under | `HKCU\...\Uninstall\Cleanup Assist` | `HKLM` |
| Install directory property | `/D=` | `INSTALLDIR` |

The NSIS installer is the better submission: it installs silently for the current
user with no UAC prompt, which is what Store-managed installation expects. The MSI
needs elevation, so a silent install will fail outright in a non-elevated context
rather than prompting.

---

## Notes for certification

Seen only by Microsoft's certification testers, never by customers. Keep it
concise and focused on how to exercise the app.

```
No account, licence key, trial code or network connection is required. The app is free, with no in-app purchases, no subscriptions and no advertising.

TESTING IN TWO MINUTES
1. Launch the app. Click "Browse..." and pick any folder holding a few hundred megabytes (Downloads or Documents is ideal), or type a path into the box. Click Scan.
2. Progress appears immediately: live counts of files, folders and bytes as they are found, plus the folder currently being read. Results then list the largest folders, each with a coloured safety badge.
3. Click any badge chip above the results to filter to that category, and again to clear it. Hovering a badge explains what that verdict means and how to reclaim the folder.

WHY THE APP READS WHAT IT READS
- File and folder metadata (names, paths, sizes, timestamps) for the location you choose to scan. File contents are never opened or read.
- The HKLM and HKCU uninstall registry keys, plus the WindowsApps and AppData\Packages folders. These are read only to name which installed application owns a folder, so results can say "Installed app - Blender" instead of showing a bare path. Nothing is written to the registry.

The application makes no network connections of any kind - no telemetry, no analytics, no crash reporting, no update check. There is no server component, so nothing external needs to be available to test it.

TESTING THE MOVE FEATURE (optional, needs two drives)
"Move..." copies a folder to another drive and leaves an NTFS junction at the original path, so software referencing the old location keeps working. To try it safely: create a small folder containing a few files on C:, scan its parent folder, then move that folder to a second drive. Afterwards the original path still lists the files, and "dir" on its parent shows the entry as <JUNCTION>. The original is retained until the copy and the junction are both verified, and any failure rolls back automatically.

EXPECTED BEHAVIOUR THAT MAY LOOK LIKE A FAULT
- Scanning a whole drive can take several minutes on a large or mechanical disk. The live counters keep updating throughout; the app is not frozen.
- A "skipped" figure in the results summary is normal. Those are folders the current account lacks permission to read. Running as administrator reduces it. Scans never abort because of them.
- Folders under C:\Windows are labelled "OS" and deliberately cannot be moved or deleted from within the app; it points to Storage Sense and Disk Cleanup instead.
- The app requires no elevation for its core features. Administrator rights only widen what it can see.

Questions during certification: https://github.com/RGaskinLtd/cleanup-assist/issues
```

---

## Before you submit — open items

1. **Code signing.** Store submissions of traditional desktop installers are
   expected to be signed by a trusted certificate. The current builds are
   unsigned, so resolve this first — Azure Trusted Signing is the cheapest route.
2. **Packaging.** The Store accepts either an MSIX package or a traditional
   EXE/MSI installer. Confirm which path you want; Tauri produces NSIS and MSI
   today, and MSIX needs extra tooling.
3. **Privacy policy must be live.** The URL is a required field and has to
   resolve before certification, so deploy `site/` first.
4. **Developer account.** Partner Center registration carries a one-off fee, and
   an individual account differs from a company account in what it can publish.

Verify current Store certification policy in Partner Center before submitting —
these requirements change.
