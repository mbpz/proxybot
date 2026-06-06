// ============================================================
// Vision Tab
// ============================================================

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { SkeletonTable } from "../ui/skeleton";
import { Eye } from "lucide-react";
import type { VisionAnalysis } from "./types";

export function VisionTab() {
  const [sessionId, setSessionId] = useState("");
  const [analyses, setAnalyses] = useState<VisionAnalysis[]>([]);
  const [uploading, setUploading] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fuseResult, setFuseResult] = useState<string | null>(null);

  async function loadAnalyses() {
    if (!sessionId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<VisionAnalysis[]>("get_vision_analyses", {
        session_id: sessionId,
      });
      setAnalyses(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function handleUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file || !sessionId) return;

    try {
      setUploading(true);
      setError(null);
      const reader = new FileReader();
      const base64 = await new Promise<string>((resolve, reject) => {
        reader.onload = () => {
          const result = reader.result as string;
          resolve(result.split(",")[1]); // strip data:image/...;base64,
        };
        reader.onerror = reject;
        reader.readAsDataURL(file);
      });

      await invoke("analyze_screenshot_base64", {
        session_id: sessionId,
        image_data_base64: base64,
        filename: file.name,
      });
      loadAnalyses();
    } catch (err) {
      setError(String(err));
    } finally {
      setUploading(false);
    }
  }

  async function handleFuse() {
    if (!sessionId) return;
    try {
      setLoading(true);
      const result = await invoke<string>("fuse_vision_with_api", {
        session_id: sessionId,
      });
      setFuseResult(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function handleDelete(id: number) {
    try {
      await invoke("delete_vision_analysis", { id });
      loadAnalyses();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div>
      <div className="flex items-end gap-3 mb-4 flex-wrap">
        <div className="flex flex-col gap-1" style={{ minWidth: 200 }}>
          <label className="text-xs text-text-muted font-mono">Session ID</label>
          <input
            type="text"
            value={sessionId}
            onChange={(e) => setSessionId(e.target.value)}
            placeholder="session_001"
          />
        </div>
        <label className="btn btn-primary cursor-pointer">
          <Eye size={16} />
          {uploading ? "Uploading..." : "Upload Screenshot"}
          <input
            type="file"
            accept="image/*"
            onChange={handleUpload}
            className="hidden"
            disabled={uploading || !sessionId}
          />
        </label>
        <Button variant="secondary" size="sm" onClick={async () => {
          const path = prompt("Enter screenshot file path:");
          if (path && sessionId) {
            try { setUploading(true); setError(null); await invoke("analyze_screenshot", { session_id: sessionId, image_path: path }); loadAnalyses(); }
            catch (err) { setError(String(err)); }
            finally { setUploading(false); }
          }
        }} disabled={!sessionId}>
          From Path
        </Button>
        <Button variant="secondary" size="sm" onClick={loadAnalyses} disabled={!sessionId}>
          Load Analyses
        </Button>
        <Button variant="secondary" size="sm" onClick={handleFuse} disabled={!sessionId}>
          Fuse with API
        </Button>
      </div>

      {error && <div className="error-banner mb-4"><span className="error-banner-message">{error}</span></div>}

      {fuseResult && (
        <div className="card mb-4" style={{ borderColor: "var(--accent-purple)" }}>
          <div className="card-header">
            <span className="card-title">Fused Component Tree</span>
          </div>
          <pre style={{
            background: "var(--bg-primary)",
            padding: "var(--space-3)",
            borderRadius: "var(--radius-md)",
            fontSize: "var(--text-xs)",
            fontFamily: "var(--font-mono)",
            maxHeight: 300,
            overflowY: "auto",
            whiteSpace: "pre-wrap",
          }}>
            {fuseResult}
          </pre>
        </div>
      )}

      {loading ? (
        <SkeletonTable rows={3} />
      ) : analyses.length > 0 ? (
        <div style={{ maxHeight: 400, overflowY: "auto" }}>
          {analyses.map((a) => (
            <div key={a.id} className="card mb-3">
              <div className="flex items-center justify-between mb-2">
                <div>
                  <span className="text-sm font-medium">{a.filename}</span>
                  <span className="text-xs text-text-muted ml-3">
                    Score: {(a.score * 100).toFixed(0)}%
                  </span>
                </div>
                <Button variant="ghost" size="sm" onClick={() => handleDelete(a.id)}>✕</Button>
              </div>
              <div className="flex flex-wrap gap-1">
                {a.components.slice(0, 10).map((c, i) => (
                  <span
                    key={i}
                    className="badge"
                    style={{
                      background: "rgba(155,93,229,0.15)",
                      color: "var(--accent-purple)",
                    }}
                  >
                    {c.component_type}
                  </span>
                ))}
                {a.components.length > 10 && (
                  <span className="text-xs text-text-muted">+{a.components.length - 10} more</span>
                )}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="empty-state">
          <Eye size={48} className="empty-state-icon" />
          <div className="empty-state-title">No vision analyses</div>
          <div className="empty-state-description">
            Upload a screenshot of a mobile app to analyze its UI components.
          </div>
        </div>
      )}
    </div>
  );
}
