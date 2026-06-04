import { Input } from "../ui/Input";
import { Button } from "../ui/Button";

interface DeployFormProps {
  sessionId: string;
  projectName: string;
  initGit: boolean;
  generating: boolean;
  onSessionIdChange: (v: string) => void;
  onProjectNameChange: (v: string) => void;
  onInitGitChange: (v: boolean) => void;
  onGenerate: () => void;
}

export function DeployForm({
  sessionId,
  projectName,
  initGit,
  generating,
  onSessionIdChange,
  onProjectNameChange,
  onInitGitChange,
  onGenerate,
}: DeployFormProps) {
  return (
    <div className="deploy-form">
      <div className="deploy-form-row">
        <label className="deploy-form-label">Session ID</label>
        <Input
          value={sessionId}
          onChange={(e) => onSessionIdChange(e.target.value)}
          placeholder="e.g. 2026-06-04-001"
          disabled={generating}
        />
      </div>
      <div className="deploy-form-row">
        <label className="deploy-form-label">Project Name</label>
        <Input
          value={projectName}
          onChange={(e) => onProjectNameChange(e.target.value)}
          placeholder="proxybot_deployment"
          disabled={generating}
        />
      </div>
      <div className="deploy-form-row">
        <label className="deploy-form-label">Output Path</label>
        <code className="deploy-form-path">~/.proxybot/deployments/{projectName || "proxybot_deployment"}</code>
      </div>
      <div className="deploy-form-row deploy-form-checkbox-row">
        <label className="deploy-form-checkbox">
          <input
            type="checkbox"
            checked={initGit}
            onChange={(e) => onInitGitChange(e.target.checked)}
            disabled={generating}
          />
          <span>Initialize git repo on write</span>
        </label>
      </div>
      <div className="deploy-form-actions">
        <Button
          variant="primary"
          onClick={onGenerate}
          disabled={generating || !sessionId.trim()}
        >
          {generating ? "Generating..." : "Generate Preview"}
        </Button>
      </div>
    </div>
  );
}
