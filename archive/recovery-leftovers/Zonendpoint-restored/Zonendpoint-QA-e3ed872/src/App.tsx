import { useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
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
type ScanLifecycle = "scanning" | "cancelling" | "idle";
type ScanStateEvent = { scan_id: string; state: ScanLifecycle };
type Theme = "light" | "dark";

const THEME_KEY = "zonendpoint-theme";

function loadTheme(): Theme {
  let theme: Theme = "light";
  try {
    theme = window.localStorage.getItem(THEME_KEY) === "dark" ? "dark" : "light";
  } catch {
    // Keep the default theme if WebView storage is unavailable.
  }
  document.documentElement.dataset.theme = theme;
  return theme;
}

const labels: Record<Status, string> = {
  CLEAN: "Clean in this scan",
  INFECTED: "Threat detected",
  UNKNOWN: "Unknown",
  ERROR: "Scan error",
  SKIPPED: "Skipped",
  CANCELLED: "Cancelled",
};

function App() {
  const [theme, setTheme] = useState<Theme>(loadTheme);
  const [selectedTarget, setSelectedTarget] = useState<SelectedTarget | null>(null);
  const [scanId, setScanId] = useState<string | null>(null);
  const [scanState, setScanState] = useState<ScanLifecycle>("idle");
  const [scanEventsReady, setScanEventsReady] = useState(false);
  const [startPending, setStartPending] = useState(false);
  const [cancelPending, setCancelPending] = useState(false);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [summary, setSummary] = useState<Summary | null>(null);
  const [error, setError] = useState("");
  const activeScanId = useRef<string | null>(null);
  const startPendingRef = useRef(false);
  const cancelPendingRef = useRef(false);
  const terminalScanId = useRef<string | null>(null);
  const stoppingVisibleUntil = useRef(0);

  useEffect(() => {
    void invoke("set_window_theme_icon", { theme }).catch((reason) => {
      console.error("Could not update the window icon", reason);
    });
  }, [theme]);

  useEffect(() => {
    let disposed = false;
    let unlistenProgress: UnlistenFn | undefined;
    let unlistenState: UnlistenFn | undefined;
    void Promise.all([
      listen<Progress>("scan-progress", ({ payload }) => {
        if (payload.scan_id === activeScanId.current) setProgress(payload);
      }),
      listen<ScanStateEvent>("scan-state", ({ payload }) => {
        if (payload.scan_id !== activeScanId.current) return;
        setScanState(payload.state);
        if (payload.state === "scanning") setStartPending(false);
        if (payload.state === "idle") terminalScanId.current = payload.scan_id;
      }),
    ]).then(([stopProgress, stopState]) => {
      if (disposed) {
        stopProgress();
        stopState();
      } else {
        unlistenProgress = stopProgress;
        unlistenState = stopState;
        setScanEventsReady(true);
      }
    }).catch((reason: unknown) => {
      if (!disposed) setError(String(reason));
    });
    return () => {
      disposed = true;
      unlistenProgress?.();
      unlistenState?.();
    };
  }, []);

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
    if (!selectedTarget || activeScanId.current || startPendingRef.current) return;
    const id = crypto.randomUUID();
    activeScanId.current = id;
    terminalScanId.current = null;
    startPendingRef.current = true;
    cancelPendingRef.current = false;
    setScanId(id);
    setScanState("idle");
    setStartPending(true);
    setCancelPending(false);
    setError("");
    setSummary(null);
    setProgress(null);
    try {
      const result = await invoke<Summary>("scan_path", { scanId: id, targetId: selectedTarget.id });
      if (activeScanId.current === id) setSummary(result);
    } catch (reason) {
      if (activeScanId.current === id) setError(String(reason));
    } finally {
      if (activeScanId.current === id) {
        const remaining = stoppingVisibleUntil.current - performance.now();
        if (remaining > 0) {
          await new Promise<void>((resolve) => window.setTimeout(resolve, remaining));
        }
        activeScanId.current = null;
        startPendingRef.current = false;
        cancelPendingRef.current = false;
        stoppingVisibleUntil.current = 0;
        setScanId(null);
        setScanState("idle");
        setStartPending(false);
        setCancelPending(false);
      }
    }
  }

  async function toggleScan() {
    if (!scanId) {
      await startScan();
      return;
    }
    const id = activeScanId.current;
    if (!id || scanState !== "scanning" || cancelPendingRef.current) return;
    cancelPendingRef.current = true;
    stoppingVisibleUntil.current = performance.now() + 250;
    const cancellation = invoke<boolean>("cancel_scan", { scanId: id }).then(
      (accepted) => ({ accepted }),
      (reason: unknown) => ({ reason }),
    );
    flushSync(() => setCancelPending(true));
    try {
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      const outcome = await cancellation;
      if ("reason" in outcome) throw outcome.reason;
      const accepted = outcome.accepted;
      if (activeScanId.current === id && terminalScanId.current !== id) {
        if (accepted) setScanState("cancelling");
        else {
          cancelPendingRef.current = false;
          setCancelPending(false);
        }
      }
    } catch (reason) {
      if (activeScanId.current === id && terminalScanId.current !== id) {
        cancelPendingRef.current = false;
        setCancelPending(false);
        setError(String(reason));
      }
    }
  }

  const latest = progress?.latest;
  const engineOffline = latest?.status === "ERROR" && /clamd|timed out|connect/i.test(latest.reason ?? "");
  const nextTheme: Theme = theme === "light" ? "dark" : "light";
  const scanButtonLabel = scanState === "cancelling" || cancelPending
    ? "Stopping…"
    : scanState === "scanning"
      ? "Stop"
      : startPending
        ? "Starting…"
        : "Start scan";
  const scanButtonDisabled = scanState === "cancelling"
    || cancelPending
    || startPending
    || (scanState === "idle" && (!!scanId || !selectedTarget || !scanEventsReady));

  function toggleTheme() {
    document.documentElement.dataset.theme = nextTheme;
    try {
      window.localStorage.setItem(THEME_KEY, nextTheme);
    } catch {
      // The current choice still applies for this session.
    }
    setTheme(nextTheme);
  }

  return (
    <main className="app-shell">
      <div className="topbar">
        <img
          className="brand-logo"
          src={theme === "dark" ? "/brand/logo-dark-slogan-transparent.png" : "/brand/logo-light-slogan-transparent.png"}
          alt="ZonEndpoint"
        />
        <button
          className="theme-toggle"
          type="button"
          title={`Switch to ${nextTheme} theme`}
          aria-label={`Switch to ${nextTheme} theme`}
          onClick={toggleTheme}
        >
          {theme === "light" ? (
            <svg aria-hidden="true" viewBox="0 0 24 24"><path d="M20.1 15.1A8.3 8.3 0 0 1 8.9 3.9 8.5 8.5 0 1 0 20.1 15.1Z" /></svg>
          ) : (
            <svg aria-hidden="true" viewBox="0 0 24 24"><circle cx="12" cy="12" r="4" /><path d="M12 2v2m0 16v2M4.93 4.93l1.42 1.42m11.3 11.3 1.42 1.42M2 12h2m16 0h2M4.93 19.07l1.42-1.42m11.3-11.3 1.42-1.42" /></svg>
          )}
        </button>
      </div>
      <header>
        <p className="eyebrow">ZONENDPOINT · ON-DEMAND SCANNING</p>
        <h1>Scan files and folders</h1>
        <p className="intro">Results show what ClamAV detected during this scan. Zonendpoint does not replace Defender or other endpoint protection.</p>
      </header>

      <section className="panel" aria-label="Scan selection">
        <div className="actions">
          <button onClick={() => void select("file")} disabled={!!scanId}>Select file</button>
          <button onClick={() => void select("folder")} disabled={!!scanId}>Select folder</button>
          <button className="primary" onClick={() => void toggleScan()} disabled={scanButtonDisabled}>{scanButtonLabel}</button>
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
