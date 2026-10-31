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
  CLEAN: "Sạch theo lần quét này",
  INFECTED: "Phát hiện mối đe dọa",
  UNKNOWN: "Chưa rõ",
  ERROR: "Lỗi quét",
  SKIPPED: "Đã bỏ qua",
  CANCELLED: "Đã hủy",
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
        <p className="eyebrow">ZONENDPOINT · QUÉT BỔ SUNG</p>
        <h1>Quét tệp và thư mục</h1>
        <p className="intro">Kết quả phản ánh phạm vi quét của ClamAV ở thời điểm này; không thay thế Defender hoặc phần mềm bảo vệ endpoint hiện có.</p>
      </header>

      <section className="panel" aria-label="Chọn nội dung quét">
        <div className="actions">
          <button onClick={() => void select("file")} disabled={!!scanId}>Chọn tệp</button>
          <button onClick={() => void select("folder")} disabled={!!scanId}>Chọn thư mục</button>
        <button className="primary" onClick={() => void startScan()} disabled={!selectedTarget || !!scanId}>Bắt đầu quét</button>
          {scanId && <button className="danger" onClick={() => void cancelScan()}>Hủy quét</button>}
        </div>
        <p className="selected-path">{selectedTarget?.path || "Chưa chọn tệp hoặc thư mục"}</p>
        <p className="guard-note">Bảo vệ thời gian thực chưa được bật trong lát cắt MVP này.</p>
        <p className="guard-note">ClamAV phải ở loopback hoặc mạng riêng/VPN tin cậy; app không gửi tệp ra dịch vụ công cộng.</p>
      </section>

      {error && <p role="alert" className="notice error">{error}</p>}
      {engineOffline && <p role="status" className="notice warning">Không kết nối được ClamAV. Kết quả lỗi/không quét được không được xem là sạch.</p>}

      {scanId && (
        <section className="panel" aria-live="polite">
          <h2>Đang quét</h2>
          <p>Đã xử lý {progress?.scanned ?? 0} mục · sạch {progress?.clean ?? 0} · phát hiện {progress?.infected ?? 0} · lỗi {progress?.errors ?? 0} · bỏ qua {progress?.skipped ?? 0}</p>
          {latest && <p className={`status ${latest.status.toLowerCase()}`}>{labels[latest.status]} — {latest.path}</p>}
        </section>
      )}

      {summary && (
        <section className="panel" aria-live="polite">
          <h2>{summary.cancelled ? "Đã hủy quét" : summary.complete ? "Quét hoàn tất" : "Quét chưa đầy đủ"}</h2>
          <p>Đã xử lý {summary.scanned} mục · sạch {summary.clean} · phát hiện {summary.infected} · chưa rõ {summary.unknown} · lỗi {summary.errors} · bỏ qua {summary.skipped} · đã hủy {summary.cancelled_files}</p>
          {!summary.complete && <p className="notice warning">Không xem phạm vi chưa hoàn tất là sạch.</p>}
          {summary.details_truncated && <p>Danh sách chi tiết đã giới hạn để giữ mức dùng bộ nhớ ổn định.</p>}
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
