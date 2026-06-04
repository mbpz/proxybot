import { Button } from "../ui/Button";
import type { DeploymentResult } from "./types";

interface DeployResultProps {
  result: DeploymentResult | null;
  error: string | null;
  onRetry?: () => void;
  onDismiss?: () => void;
}

export function DeployResult({ result, error, onRetry, onDismiss }: DeployResultProps) {
  if (error) {
    return (
      <div className="error-banner" style={{ margin: "0 var(--space-4) var(--space-2)" }}>
        <span className="error-banner-message">{error}</span>
        {onRetry && (
          <Button variant="secondary" size="sm" onClick={onRetry}>
            Retry
          </Button>
        )}
      </div>
    );
  }

  if (!result) return null;

  return (
    <div className="deploy-result deploy-result-success">
      <div className="deploy-result-header">
        <span className="deploy-result-icon">✓</span>
        <span className="deploy-result-title">Deployment bundle ready</span>
        {onDismiss && (
          <button
            className="deploy-result-dismiss"
            onClick={onDismiss}
            aria-label="Dismiss"
          >
            ×
          </button>
        )}
      </div>
      <pre className="deploy-result-message">{result.message}</pre>
    </div>
  );
}
