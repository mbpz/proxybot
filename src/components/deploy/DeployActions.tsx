import { Button } from "../ui/Button";

interface DeployActionsProps {
  bundlePath: string;
  hasBundle: boolean;
  writing: boolean;
  onWrite: () => void;
  onReinitGit: () => void;
}

export function DeployActions({
  bundlePath,
  hasBundle,
  writing,
  onWrite,
  onReinitGit,
}: DeployActionsProps) {
  return (
    <div className="deploy-actions">
      <div className="deploy-actions-path-row">
        <span className="deploy-actions-label">Bundle path:</span>
        <code className="deploy-actions-path">{bundlePath || "(not yet written)"}</code>
      </div>
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
