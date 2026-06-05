import { Button } from "../ui/Button";

interface DeployActionsProps {
  bundlePath: string;
  hasBundle: boolean;
  writing: boolean;
  lastGitInitAt: string | null;
  onWrite: () => void;
  onReinitGit: () => void;
}

// Format an ISO 8601 timestamp for display. Kept simple — locale string.
function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString();
}

export function DeployActions({
  bundlePath,
  hasBundle,
  writing,
  lastGitInitAt,
  onWrite,
  onReinitGit,
}: DeployActionsProps) {
  return (
    <div className="deploy-actions">
      <div className="deploy-actions-path-row">
        <span className="deploy-actions-label">Bundle path:</span>
        <code className="deploy-actions-path">{bundlePath || "(not yet written)"}</code>
      </div>
      {lastGitInitAt && (
        <div className="deploy-actions-path-row">
          <span className="deploy-actions-label">Git status:</span>
          <span className="deploy-actions-git-info">
            Git initialized at {formatTimestamp(lastGitInitAt)}
          </span>
        </div>
      )}
      <div className="deploy-actions-buttons">
        <Button
          variant="primary"
          onClick={onWrite}
          disabled={!hasBundle || writing}
        >
          {writing ? "Writing..." : "Write to Disk"}
        </Button>
        <Button
          variant="secondary"
          onClick={onReinitGit}
          disabled={!bundlePath || writing}
        >
          Re-init Git
        </Button>
      </div>
    </div>
  );
}
