import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ask, message, open } from "@tauri-apps/plugin-dialog";
import "./App.css";

type Tier =
  | "os-critical"
  | "app-installed"
  | "app-active"
  | "app-stale"
  | "known-reclaimable"
  | "user-content"
  | "unknown";

interface DirStat {
  path: string;
  bytes: number;
  tier: Tier;
  ruleId: string | null;
  owner: string | null;
  note: string | null;
}

interface FileStat {
  path: string;
  bytes: number;
}

interface ScanResult {
  root: string;
  totalBytes: number;
  fileCount: number;
  dirCount: number;
  skipped: number;
  durationMs: number;
  topDirs: DirStat[];
  largestFiles: FileStat[];
}

interface RelocateReport {
  src: string;
  dest: string;
  bytesMoved: number;
  filesMoved: number;
  linksSkipped: number;
  backupLeftAt: string | null;
}

interface RelocateProgress {
  phase: "preparing" | "copying" | "finalizing";
  bytesDone: number;
  bytesTotal: number;
  currentFile: string;
}

interface ScanProgress {
  phase: "walking" | "aggregating";
  files: number;
  dirs: number;
  bytes: number;
  currentPath: string;
}

const TIER_LABELS: Record<Tier, string> = {
  "os-critical": "OS",
  "app-installed": "Installed app",
  "app-active": "Active app",
  "app-stale": "Stale app",
  "known-reclaimable": "Reclaimable",
  "user-content": "Your files",
  unknown: "Unclassified",
};

const LEGEND: { tier: Tier; text: string }[] = [
  { tier: "os-critical", text: "Windows itself — leave to Storage Sense / Disk Cleanup" },
  { tier: "app-installed", text: "Belongs to an installed app — uninstalling reclaims it" },
  { tier: "known-reclaimable", text: "Caches and temp data — safe to reclaim (hover for how)" },
  { tier: "user-content", text: "Your personal files — only you can judge" },
  { tier: "unknown", text: "Nothing claimed it — investigate before touching" },
];

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log2(bytes) / 10), units.length - 1);
  const value = bytes / 2 ** (10 * i);
  return `${value >= 100 ? value.toFixed(0) : value.toFixed(1)} ${units[i]}`;
}

function progressPct(p: RelocateProgress | null): number {
  if (!p || p.bytesTotal === 0) return 0;
  return Math.min(100, (p.bytesDone / p.bytesTotal) * 100);
}

function progressLabel(p: RelocateProgress | null): string {
  if (!p || p.phase === "preparing") return "Preparing…";
  if (p.phase === "finalizing") return "Finishing…";
  return `${progressPct(p).toFixed(0)}% of ${formatBytes(p.bytesTotal)}`;
}

function App() {
  const [drives, setDrives] = useState<string[]>([]);
  const [path, setPath] = useState("C:\\");
  const [scanning, setScanning] = useState(false);
  const [result, setResult] = useState<ScanResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [movingPath, setMovingPath] = useState<string | null>(null);
  const [movedPaths, setMovedPaths] = useState<Map<string, string>>(new Map());
  const [moveProgress, setMoveProgress] = useState<RelocateProgress | null>(null);
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);
  const [tierFilter, setTierFilter] = useState<Set<Tier>>(new Set());

  useEffect(() => {
    invoke<string[]>("list_drives").then(setDrives).catch(() => {});
    const unlistenMove = listen<RelocateProgress>("relocate-progress", (e) =>
      setMoveProgress(e.payload)
    );
    const unlistenScan = listen<ScanProgress>("scan-progress", (e) =>
      setScanProgress(e.payload)
    );
    return () => {
      unlistenMove.then((f) => f());
      unlistenScan.then((f) => f());
    };
  }, []);

  async function runScan() {
    setScanning(true);
    setError(null);
    setResult(null);
    setMovedPaths(new Map());
    setScanProgress(null);
    setTierFilter(new Set());
    try {
      setResult(await invoke<ScanResult>("scan_path", { path }));
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
      setScanProgress(null);
    }
  }

  async function browseForPath() {
    const picked = await open({
      directory: true,
      title: "Choose a drive or folder to scan",
      defaultPath: path,
    });
    if (typeof picked === "string") setPath(picked);
  }

  function toggleTier(tier: Tier) {
    setTierFilter((prev) => {
      const next = new Set(prev);
      if (next.has(tier)) next.delete(tier);
      else next.add(tier);
      return next;
    });
  }

  async function moveDir(dir: DirStat) {
    const destParent = await open({
      directory: true,
      title: `Move "${dir.path}" to…`,
    });
    if (typeof destParent !== "string") return;

    const confirmed = await ask(
      `Move ${dir.path} (${formatBytes(dir.bytes)}) to ${destParent}?\n\n` +
        "A junction will be left at the old location, so apps using the " +
        "old path keep working. Nothing is deleted until the copy and " +
        "junction are verified.\n\n" +
        "Close any programs using this folder first (including Explorer " +
        "windows open on it) — Windows blocks the move while files inside " +
        "are in use.",
      { title: "Move folder", kind: "warning" }
    );
    if (!confirmed) return;

    setMovingPath(dir.path);
    setMoveProgress(null);
    try {
      const report = await invoke<RelocateReport>("relocate_dir", {
        src: dir.path,
        destParent,
      });
      setMovedPaths((prev) => new Map(prev).set(dir.path, report.dest));
      let summary =
        `Moved ${report.filesMoved.toLocaleString()} files ` +
        `(${formatBytes(report.bytesMoved)}) to ${report.dest}.`;
      if (report.linksSkipped > 0) {
        summary += `\n${report.linksSkipped} links inside were not copied.`;
      }
      if (report.backupLeftAt) {
        summary +=
          `\nSome original files were locked; delete manually to free space:\n` +
          report.backupLeftAt;
      }
      await message(summary, { title: "Move complete" });
    } catch (e) {
      await message(String(e), { title: "Move failed", kind: "error" });
    } finally {
      setMovingPath(null);
      setMoveProgress(null);
    }
  }

  const maxDirBytes = result?.topDirs[0]?.bytes ?? 1;
  const tierCounts = new Map<Tier, number>();
  for (const dir of result?.topDirs ?? []) {
    tierCounts.set(dir.tier, (tierCounts.get(dir.tier) ?? 0) + 1);
  }
  const visibleDirs = (result?.topDirs ?? []).filter(
    (d) => tierFilter.size === 0 || tierFilter.has(d.tier)
  );

  return (
    <main className="app">
      <header className="header">
        <h1>Cleanup Assist</h1>
        <p className="tagline">
          Where your disk space went — and whether it's safe to take back
        </p>
      </header>

      <section className="controls">
        <select
          value={drives.includes(path) ? path : ""}
          onChange={(e) => e.target.value && setPath(e.target.value)}
          disabled={scanning}
        >
          <option value="">Drive…</option>
          {drives.map((d) => (
            <option key={d} value={d}>
              {d}
            </option>
          ))}
        </select>
        <input
          type="text"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="Path to scan, e.g. C:\"
          disabled={scanning}
          onKeyDown={(e) => e.key === "Enter" && !scanning && runScan()}
        />
        <button
          className="secondary"
          onClick={browseForPath}
          disabled={scanning}
          title="Pick a folder with the file explorer"
        >
          Browse…
        </button>
        <button onClick={runScan} disabled={scanning || !path.trim()}>
          {scanning ? "Scanning…" : "Scan"}
        </button>
      </section>

      {scanning && (
        <section className="scan-progress">
          <div className="indeterminate">
            <span />
          </div>
          <div className="scan-stats">
            <span className="scan-stat">
              <b>{(scanProgress?.files ?? 0).toLocaleString()}</b> files
            </span>
            <span className="scan-stat">
              <b>{formatBytes(scanProgress?.bytes ?? 0)}</b> found
            </span>
            <span className="scan-stat">
              <b>{(scanProgress?.dirs ?? 0).toLocaleString()}</b> folders
            </span>
          </div>
          <div className="scan-current">
            {scanProgress?.phase === "aggregating"
              ? "Crunching the numbers…"
              : scanProgress?.currentPath || `Starting in ${path}…`}
          </div>
        </section>
      )}
      {error && <p className="status error">{error}</p>}

      {result && (
        <>
          <section className="summary">
            <div className="stat">
              <span className="stat-value">{formatBytes(result.totalBytes)}</span>
              <span className="stat-label">scanned</span>
            </div>
            <div className="stat">
              <span className="stat-value">
                {result.fileCount.toLocaleString()}
              </span>
              <span className="stat-label">files</span>
            </div>
            <div className="stat">
              <span className="stat-value">
                {result.dirCount.toLocaleString()}
              </span>
              <span className="stat-label">folders</span>
            </div>
            <div className="stat">
              <span className="stat-value">
                {(result.durationMs / 1000).toFixed(1)}s
              </span>
              <span className="stat-label">scan time</span>
            </div>
            {result.skipped > 0 && (
              <div className="stat">
                <span className="stat-value">
                  {result.skipped.toLocaleString()}
                </span>
                <span className="stat-label">skipped (no access)</span>
              </div>
            )}
          </section>

          <section className="legend">
            {LEGEND.map(({ tier, text }) => (
              <button
                key={tier}
                className={`chip tier-${tier}${tierFilter.has(tier) ? " active" : ""}`}
                title={text}
                onClick={() => toggleTier(tier)}
              >
                {TIER_LABELS[tier]}
                <span className="chip-count">{tierCounts.get(tier) ?? 0}</span>
              </button>
            ))}
            {tierFilter.size > 0 && (
              <button className="chip clear" onClick={() => setTierFilter(new Set())}>
                clear filter
              </button>
            )}
            <span className="legend-hint">click to filter · hover for meaning</span>
          </section>

          <section>
            <h2>
              Largest directories
              {tierFilter.size > 0 &&
                ` (${visibleDirs.length} of ${result.topDirs.length})`}
            </h2>
            {visibleDirs.length === 0 && (
              <p className="status">No directories match this filter.</p>
            )}
            <ul className="dir-list">
              {visibleDirs.map((dir) => {
                const movedTo = movedPaths.get(dir.path);
                const badgeText = dir.owner
                  ? `${TIER_LABELS[dir.tier]} · ${dir.owner}`
                  : dir.ruleId
                    ? `${TIER_LABELS[dir.tier]} · ${dir.ruleId}`
                    : TIER_LABELS[dir.tier];
                return (
                  <li key={dir.path} className="dir-row">
                    <div
                      className={`bar tier-${dir.tier}`}
                      style={{ width: `${(dir.bytes / maxDirBytes) * 100}%` }}
                    />
                    <span className="dir-path" title={dir.path}>
                      {dir.path}
                    </span>
                    {movedTo ? (
                      <span className="badge moved" title={`Now at ${movedTo}`}>
                        moved ✓
                      </span>
                    ) : movingPath === dir.path ? (
                      <span
                        className="move-progress"
                        title={moveProgress?.currentFile ?? ""}
                      >
                        <span className="progress-track">
                          <span
                            className="progress-fill"
                            style={{ width: `${progressPct(moveProgress)}%` }}
                          />
                        </span>
                        <span className="progress-text">
                          {progressLabel(moveProgress)}
                        </span>
                      </span>
                    ) : (
                      dir.tier !== "os-critical" && (
                        <button
                          className="move-btn"
                          disabled={movingPath !== null}
                          onClick={() => moveDir(dir)}
                          title="Move to another drive; a junction keeps the old path working"
                        >
                          Move…
                        </button>
                      )
                    )}
                    <span
                      className={`badge tier-${dir.tier}`}
                      title={dir.note ?? "No classification — investigate before touching"}
                    >
                      {badgeText}
                    </span>
                    <span className="dir-size">{formatBytes(dir.bytes)}</span>
                  </li>
                );
              })}
            </ul>
          </section>

          <section>
            <h2>Largest files</h2>
            <ul className="dir-list">
              {result.largestFiles.map((file) => (
                <li key={file.path} className="dir-row">
                  <span className="dir-path" title={file.path}>
                    {file.path}
                  </span>
                  <span className="dir-size">{formatBytes(file.bytes)}</span>
                </li>
              ))}
            </ul>
          </section>
        </>
      )}

      {!result && !scanning && !error && (
        <p className="status">Pick a drive or folder and hit Scan.</p>
      )}
    </main>
  );
}

export default App;
