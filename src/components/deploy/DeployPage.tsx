import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { DeployForm } from "./DeployForm";
import { DeployPreview } from "./DeployPreview";
import { DeployActions } from "./DeployActions";
import { DeployResult } from "./DeployResult";
import "./DeployPage.css";
import type {
  DeploymentBundle,
  DeploymentResult as DeploymentResultT,
  DeploymentRecord,
  DeployTab,
} from "./types";

export function DeployPage() {
  // Inputs
  const [sessionId, setSessionId] = useState("");
  const [projectName, setProjectName] = useState("proxybot_deployment");
  const [initGit, setInitGit] = useState(true);

  // Bundle + preview
  const [bundle, setBundle] = useState<DeploymentBundle | null>(null);
  const [activeTab, setActiveTab] = useState<DeployTab>("compose");

  // Persistence
  const [bundlePath, setBundlePath] = useState("");

  // UI
  const [generating, setGenerating] = useState(false);
  const [writing, setWriting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<DeploymentResultT | null>(null);

  // Track which handler was last attempted (for Retry button)
  const [lastAction, setLastAction] = useState<"generate" | "write" | "reinit" | null>(null);

  // Hydrate last deployment record on mount (when sessionId/projectName set)
  useEffect(() => {
    if (!sessionId.trim()) {
      setBundlePath("");
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const rec = await invoke<DeploymentRecord | null>("get_last_deployment", {
          sessionId,
          projectName,
        });
        if (!cancelled && rec) {
          setBundlePath(rec.bundle_path);
        } else if (!cancelled) {
          setBundlePath("");
        }
      } catch (err) {
        // Non-fatal: just log
        console.error("Failed to load last deployment:", err);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [sessionId, projectName]);

  const handleGenerate = useCallback(async () => {
    if (!sessionId.trim()) {
      setError("Session ID is required");
      return;
    }
    setGenerating(true);
    setError(null);
    setLastAction("generate");
    try {
      const b = await invoke<DeploymentBundle>("generate_deployment_bundle", {
        sessionId,
        projectName: projectName || null,
      });
      setBundle(b);
    } catch (err) {
      setError(String(err));
    } finally {
      setGenerating(false);
    }
  }, [sessionId, projectName]);

  const handleWrite = useCallback(async () => {
    setWriting(true);
    setError(null);
    setLastAction("write");
    try {
      const r = await invoke<DeploymentResultT>("write_deployment_bundle", {
        sessionId,
        projectName: projectName || null,
        initGit,
      });
      setResult(r);
      setBundlePath(r.bundle_path);
    } catch (err) {
      setError(String(err));
    } finally {
      setWriting(false);
    }
  }, [sessionId, projectName, initGit]);

  const handleReinitGit = useCallback(async () => {
    if (!bundlePath) return;
    setWriting(true);
    setError(null);
    setLastAction("reinit");
    try {
      const r = await invoke<DeploymentResultT>("git_init_deployment", {
        path: bundlePath,
      });
      setResult(r);
    } catch (err) {
      setError(String(err));
    } finally {
      setWriting(false);
    }
  }, [bundlePath]);

  const lastFailedHandler =
    !error ? null :
    lastAction === "generate" ? handleGenerate :
    lastAction === "write" ? handleWrite :
    lastAction === "reinit" ? handleReinitGit :
    null;

  return (
    <div className="deploy-page">
      <div className="panel">
        <div className="panel-header">
          <span className="panel-title">Deploy</span>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => {
              setError(null);
              setResult(null);
            }}
          >
            Reset
          </Button>
        </div>

        <DeployResult
          result={result}
          error={error}
          onRetry={lastFailedHandler ?? undefined}
          onDismiss={() => {
            setError(null);
            setResult(null);
          }}
        />

        <div style={{ padding: "var(--space-4)" }}>
          <div className="deploy-section">
            <div className="deploy-section-title">Inputs</div>
            <DeployForm
              sessionId={sessionId}
              projectName={projectName}
              initGit={initGit}
              generating={generating}
              onSessionIdChange={setSessionId}
              onProjectNameChange={setProjectName}
              onInitGitChange={setInitGit}
              onGenerate={handleGenerate}
            />
          </div>

          <div className="deploy-section">
            <div className="deploy-section-title">Preview</div>
            <DeployPreview
              bundle={bundle}
              activeTab={activeTab}
              loading={generating}
              onTabChange={setActiveTab}
            />
          </div>

          <div className="deploy-section">
            <div className="deploy-section-title">Actions</div>
            <DeployActions
              bundlePath={bundlePath}
              hasBundle={bundle !== null}
              writing={writing}
              onWrite={handleWrite}
              onReinitGit={handleReinitGit}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
