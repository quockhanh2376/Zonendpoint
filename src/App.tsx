import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import "./App.css";

type Status = "CLEAN" | "INFECTED" | "UNKNOWN" | "ERROR" | "SKIPPED" | "CANCELLED";
type ScanResult = {
  path: string;
  sha256: string | null;
  status: Status;
  reason: string | null;
  engine_version: string | null;
  bytes_scanned: number;
};
type Summary = {
  complete: boolean;
  cancelled: boolean;
  scanned: number;
  clean: number;
  infected: number;
  unknown: number;
  errors: number;
  skipped: number;
  cancelled_files: number;
  details_truncated: boolean;
  results: ScanResult[];
};
type Progress = { scan_id: string; scanned: number; clean: number; infected: number; unknown: number; errors: number; skipped: number; cancelled_files: number; latest: ScanResult };
type SelectedTarget = { id: string; path: string };

const labels: Record<Status, string> = {
  CLEAN: "Clean in this scan",
  INFECTED: "Threat detected",
  UNKNOWN: "Unknown",
  ERROR: "Scan error",
  SKIPPED: "Skipped",
  CANCELLED: "Cancelled",
};

function App() {
  const [selectedTarget, setSelectedTarget] = useState<SelectedTarget | null>(null);
  const [scanId, setScanId] = useState<string | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [summary, setSummary] = useState<Summary | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    void listen<Progress>("scan-progress", ({ payload }) => {
      if (payload.scan_id === scanId) setProgress(payload);
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    }).catch((reason: unknown) => setError(String(reason)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [scanId]);

  async function select(kind: "file" | "folder") {
    setError("");
    try {
      const target = await invoke<SelectedTarget | null>("select_scan_path", { kind });
      if (target) {
        setSelectedTarget(target);
        setSummary(null);
        setProgress(null);
      }
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function startScan() {
    if (!selectedTarget || scanId) return;
    const id = crypto.randomUUID();
    setScanId(id);
    setError("");
    setSummary(null);
    setProgress(null);
    try {
      const result = await invoke<Summary>("scan_path", { scanId: id, targetId: selectedTarget.id });
      setSummary(result);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setScanId(null);
    }
  }

  async function cancelScan() {
    if (!scanId) return;
    try {
      await invoke<boolean>("cancel_scan", { scanId });
    } catch (reason) {
      setError(String(reason));
    }
  }

  const latest = progress?.latest;
  const engineOffline = latest?.status === "ERROR" && /clamd|timed out|connect/i.test(latest.reason ?? "");

  return (
    <main className="app-shell">
      <header>
        <p className="eyebrow">ZONENDPOINT · ON-DEMAND SCANNING</p>
        <h1>Scan files and folders</h1>
        <p className="intro">Results show what ClamAV detected during this scan. Zonendpoint does not replace Defender or other endpoint protection.</p>
      </header>

      <section className="panel" aria-label="Scan selection">
        <div className="actions">
          <button onClick={() => void select("file")} disabled={!!scanId}>Select file</button>
          <button onClick={() => void select("folder")} disabled={!!scanId}>Select folder</button>
          <button className="primary" onClick={() => void startScan()} disabled={!selectedTarget || !!scanId}>Start scan</button>
          {scanId && <button className="danger" onClick={() => void cancelScan()}>Cancel scan</button>}
        </div>
        <p className="selected-path">{selectedTarget?.path || "No file or folder selected"}</p>
        <p className="guard-note">Real-time protection is not enabled in this MVP.</p>
        <p className="guard-note">ClamAV must run on loopback or a trusted private/VPN network. Files are not sent to public services.</p>
      </section>

      {error && <p role="alert" className="notice error">{error}</p>}
      {engineOffline && <p role="status" className="notice warning">Could not connect to ClamAV. Errors or unscanned results must not be treated as clean.</p>}

      {scanId && (
        <section className="panel" aria-live="polite">
          <h2>Scanning</h2>
          <p>Processed {progress?.scanned ?? 0} items · clean {progress?.clean ?? 0} · threats {progress?.infected ?? 0} · unknown {progress?.unknown ?? 0} · errors {progress?.errors ?? 0} · skipped {progress?.skipped ?? 0} · cancelled {progress?.cancelled_files ?? 0}</p>
          {latest && <p className={`status ${latest.status.toLowerCase()}`}>{labels[latest.status]} — {latest.path}</p>}
        </section>
      )}

      {summary && (
        <section className="panel" aria-live="polite">
          <h2>{summary.cancelled ? "Scan cancelled" : summary.complete ? "Scan complete" : "Scan incomplete"}</h2>
          <p>Processed {summary.scanned} items · clean {summary.clean} · threats {summary.infected} · unknown {summary.unknown} · errors {summary.errors} · skipped {summary.skipped} · cancelled {summary.cancelled_files}</p>
          {!summary.complete && <p className="notice warning">Do not treat an incomplete scan as clean.</p>}
          {summary.details_truncated && <p>Result details are limited to keep memory use bounded.</p>}
          <ul className="results">
            {summary.results.slice(-50).map((item, index) => (
              <li key={`${item.path}-${index}`}>
                <span className={`status ${item.status.toLowerCase()}`}>{labels[item.status]}</span>
                <span className="result-path">{item.path}</span>
                {item.reason && <small>{item.reason}</small>}
              </li>
            ))}
          </ul>
        </section>
      )}
    </main>
  );
}

export default App;
