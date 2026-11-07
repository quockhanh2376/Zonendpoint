import { useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow, currentMonitor, LogicalSize } from "@tauri-apps/api/window";
import "./App.css";

type Status = "CLEAN" | "INFECTED" | "UNKNOWN" | "ERROR" | "SKIPPED" | "CANCELLED";
type ServerStatus = "checking" | "connected" | "disconnected";
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
  const [targetMenuOpen, setTargetMenuOpen] = useState(false);
  const targetPickerRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!targetMenuOpen) return;
    const closeOutside = (event: PointerEvent) => {
      if (!targetPickerRef.current?.contains(event.target as Node)) setTargetMenuOpen(false);
    };
    document.addEventListener("pointerdown", closeOutside);
    return () => document.removeEventListener("pointerdown", closeOutside);
  }, [targetMenuOpen]);
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
  const [serverStatus, setServerStatus] = useState<ServerStatus>("checking");
  const serverStatusRef = useRef<ServerStatus>("checking");

  const checkServer = async () => {
    try {
      await invoke<string>("check_server_status");
      serverStatusRef.current = "connected";
      setServerStatus("connected");
      return true;
    } catch {
      serverStatusRef.current = "disconnected";
      setServerStatus("disconnected");
      return false;
    }
  };

  useEffect(() => {
    void checkServer();
    const interval = window.setInterval(() => {
      if (!activeScanId.current) {
        void checkServer();
      }
    }, 30000);
    return () => window.clearInterval(interval);
  }, []);
  useEffect(() => {
    if (!summary) return;
    let disposed = false;
    const frame = requestAnimationFrame(() => {
      const desired = Math.ceil(document.querySelector<HTMLElement>(".app-shell")?.scrollHeight ?? 240);
      void (async () => {
        const appWindow = getCurrentWindow();
        const monitor = await currentMonitor();
        const position = await appWindow.outerPosition();
        const size = await appWindow.innerSize();
        const scale = await appWindow.scaleFactor();
        const available = monitor
          ? (monitor.workArea.position.y + monitor.workArea.size.height - position.y) / monitor.scaleFactor - 48
          : window.screen.availHeight - 80;
        const height = Math.max(window.innerHeight, Math.min(desired, available));
        if (!disposed && height > window.innerHeight) {
          await appWindow.setSize(new LogicalSize(size.width / scale, height));
        }
      })().catch((reason) => console.error("Could not resize scan results window", reason));
    });
    return () => { disposed = true; cancelAnimationFrame(frame); };
  }, [summary]);
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
    if (activeScanId.current || startPendingRef.current) return;
    setError("");
    setSummary(null);
    setProgress(null);
    setStartPending(true);
    startPendingRef.current = true;

    const connected = await checkServer();
    if (!connected) {
      setError("Cannot connect to ZonEndpoint server");
      setStartPending(false);
      startPendingRef.current = false;
      return;
    }

    const id = crypto.randomUUID();
    activeScanId.current = id;
    terminalScanId.current = null;
    cancelPendingRef.current = false;
    setScanId(id);
    setScanState("idle");
    setCancelPending(false);
    try {
      const result = selectedTarget
        ? await invoke<Summary>("scan_path", { scanId: id, targetId: selectedTarget.id })
        : await invoke<Summary>("quick_scan", { scanId: id });
      if (activeScanId.current === id) setSummary(result);
    } catch (reason) {
      if (activeScanId.current === id) {
        const msg = String(reason);
        if (/cannot connect|connect|offline|timed out|refused/i.test(msg)) {
          setError("Cannot connect to ZonEndpoint server");
          serverStatusRef.current = "disconnected";
          setServerStatus("disconnected");
        } else {
          setError(msg);
        }
      }
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
        setSelectedTarget(null);
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
  const isAllClean = !scanId && !startPending && !error && !!summary?.complete
    && !summary.cancelled && summary.scanned > 0 && summary.clean === summary.scanned;
  const nextTheme: Theme = theme === "light" ? "dark" : "light";
  const scanButtonLabel = scanState === "cancelling" || cancelPending
    ? "Stopping…"
    : scanState === "scanning"
      ? "Stop"
      : startPending
        ? "Starting…"
        : selectedTarget ? "Scan" : "Quick Scan";
  const scanButtonDisabled = scanState === "cancelling"
    || cancelPending
    || startPending
    || (scanState === "idle" && (!!scanId || !scanEventsReady));

  const serverTooltip =
    serverStatus === "connected"
      ? "Connected to ZonEndpoint server"
      : serverStatus === "disconnected"
        ? "Cannot connect to ZonEndpoint server. Check your connection."
        : "Checking connection…";

  const displayedResults = summary
    ? isAllClean
      ? []
      : summary.complete && !summary.cancelled && summary.infected > 0
        ? summary.results.filter((item) => item.status === "INFECTED")
        : summary.results.filter((item) => item.status !== "CLEAN")
    : [];

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
        <div className="brand-heading">
          <img
            className="brand-logo"
            src={theme === "dark" ? "/brand/logo-dark-slogan-transparent.png" : "/brand/logo-light-slogan-transparent.png"}
            alt="ZonEndpoint"
          />
          <p className="brand-slogan">Protect your digital life</p>
        </div>
        <div className="header-controls">
          <button
            type="button"
            className={`server-status status-${serverStatus}`}
            data-status={serverStatus}
            title={serverTooltip}
            aria-label={serverTooltip}
            onClick={() => void checkServer()}
            disabled={serverStatus === "checking"}
          >
            <span className="server-status-dot" aria-hidden="true" />
          </button>
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
        <section className="panel selection-panel" aria-label="Scan selection">
          <h1>Scan files and folders</h1>
          <div className="actions">
            <div className="target-picker" ref={targetPickerRef}
              onBlur={(event) => {
                if (!event.currentTarget.contains(event.relatedTarget)) setTargetMenuOpen(false);
              }}
              onKeyDown={(event) => {
                if (event.key === "Escape") {
                  setTargetMenuOpen(false);
                  targetPickerRef.current?.querySelector("button")?.focus();
                }
              }}>
              <button aria-expanded={targetMenuOpen} aria-controls="target-options"
                onClick={() => setTargetMenuOpen(!targetMenuOpen)} disabled={!!scanId}>
                Select target ▾
              </button>
              {targetMenuOpen && !scanId && (
                <div id="target-options" className="target-options" role="group" aria-label="Target type">
                  <button onClick={() => { setTargetMenuOpen(false); void select("file"); }}>File</button>
                  <button onClick={() => { setTargetMenuOpen(false); void select("folder"); }}>Folder</button>
                </div>
              )}
            </div>
            <button
              className="primary"
              onClick={() => void toggleScan()}
              disabled={scanButtonDisabled}
            >
              {scanButtonLabel}
            </button>
          </div>
          {!scanId && !summary && <p className="selected-path">{selectedTarget?.path || "Windows core files + drivers"}</p>}

          {scanId && (
            <section className="scan-feedback" aria-live="polite">
              <h2>Scanning</h2>
              <p className="selected-path">{latest?.path || selectedTarget?.path || "Windows core files + drivers"}</p>
            </section>
          )}

          {summary && (
            <section className="scan-feedback" aria-live="polite">
              {isAllClean ? (
                <div className="all-good-card">
                  <svg className="all-good-check" aria-hidden="true" viewBox="0 0 20 20">
                    <path d="m4 10 4 4 8-8" />
                  </svg>
                  <h2 className="all-good-line">All Good</h2>
                </div>
              ) : (
                <>
                  <h2>{summary.cancelled ? "Scan cancelled" : summary.complete ? "Scan complete" : "Scan incomplete"}</h2>
                  <p className="scan-stats">
                    Processed {summary.scanned} items · clean {summary.clean} · threats {summary.infected} · unknown {summary.unknown} · errors {summary.errors} · skipped {summary.skipped} · cancelled {summary.cancelled_files}
                  </p>
                  {!summary.complete && <p className="notice warning">Do not treat an incomplete scan as clean.</p>}
                  {summary.details_truncated && <p>Result details are limited to keep memory use bounded.</p>}
                  {displayedResults.length > 0 && (
                    <ul className="results">
                      {displayedResults.map((item, index) => (
                        <li key={`${item.path}-${index}`}>
                          <span className={`status ${item.status.toLowerCase()}`}>{labels[item.status]}</span>
                          <span className="result-path">{item.path}</span>
                          {item.reason && <small>{item.reason}</small>}
                        </li>
                      ))}
                    </ul>
                  )}
                </>
              )}
            </section>
          )}
          {error && <p role="alert" className="notice error">{error}</p>}
        </section>
      </div>
    </main>
  );
}

export default App;
