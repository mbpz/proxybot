import { useState, useEffect, useCallback, useRef } from "react";
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

  // Approximate timestamp of the most recent successful git init.
  // Sourced from the hydrated record on mount; updated locally after
  // a successful reinit (we don't refetch the record to keep this lightweight).
  const [lastGitInitAt, setLastGitInitAt] = useState<string | null>(null);

  // Hydrate last deployment record on mount (when sessionId/projectName set).
  // Debounced 300ms so a burst of keystrokes only fires one invoke.
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    if (!sessionId.trim()) {
      setBundlePath("");
      setLastGitInitAt(null);
      return;
    }
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    let cancelled = false;
    debounceRef.current = setTimeout(() => {
      (async () => {
        try {
          const rec = await invoke<DeploymentRecord | null>("get_last_deployment", {
            sessionId,
            projectName,
          });
          if (cancelled) return;
          if (rec) {
            setBundlePath(rec.bundle_path);
            setLastGitInitAt(rec.last_git_init_at);
          } else {
            setBundlePath("");
            setLastGitInitAt(null);
          }
        } catch (err) {
          // Non-fatal: just log
          console.error("Failed to load last deployment:", err);
        }
      })();
    }, 300);
    return () => {
      cancelled = true;
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
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
      // Approximate: stamp now rather than re-querying the record.
      setLastGitInitAt(new Date().toISOString());
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
          {/*
            "Clear status" intentionally only clears the result/error banner.
            It does NOT touch sessionId, projectName, initGit, or the generated
            bundle — those represent user intent and are not "status". Users
            can edit the inputs directly if they want to start over.
          */}
          <Button
            variant="secondary"
            size="sm"
            aria-label="Clear status messages"
            onClick={() => {
              setError(null);
              setResult(null);
            }}
          >
            Clear status
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
              lastGitInitAt={lastGitInitAt}
              onWrite={handleWrite}
              onReinitGit={handleReinitGit}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
