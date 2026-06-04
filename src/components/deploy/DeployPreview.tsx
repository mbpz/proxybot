import { Tabs } from "../ui/Tabs";
import { CodeViewer } from "../shared/CodeViewer";
import { SkeletonCard } from "../ui/skeleton";
import { ErrorBoundary } from "../ui/error-boundary";
import type { DeploymentBundle, DeployTab } from "./types";

interface DeployPreviewProps {
  bundle: DeploymentBundle | null;
  activeTab: DeployTab;
  loading: boolean;
  onTabChange: (t: DeployTab) => void;
}

export function DeployPreview({ bundle, activeTab, loading, onTabChange }: DeployPreviewProps) {
  if (loading) {
    return (
      <div className="deploy-preview">
        <SkeletonCard />
      </div>
    );
  }

  if (!bundle) {
    return (
      <div className="deploy-preview deploy-preview-empty">
        <div className="empty-state">
          <div className="empty-state-icon">🐳</div>
          <div className="empty-state-title">No preview yet</div>
          <div className="empty-state-description">
            Fill in a session ID and click <strong>Generate Preview</strong> to see the
            Docker Compose stack that would be produced.
          </div>
        </div>
      </div>
    );
  }

  const tabs = [
    { id: "compose", label: "docker-compose.yml" },
    { id: "readme", label: "README.md" },
    { id: "ci", label: "e2e.yml" },
  ];

  const content =
    activeTab === "compose"
      ? bundle.docker_compose_content
      : activeTab === "readme"
      ? bundle.readme_content
      : bundle.ci_template_content;

  const contentType =
    activeTab === "compose" ? "yaml" : activeTab === "readme" ? "markdown" : "yaml";

  return (
    <div className="deploy-preview">
      <Tabs tabs={tabs} activeTab={activeTab} onTabChange={(t) => onTabChange(t as DeployTab)} />
      <div className="deploy-preview-content">
        <ErrorBoundary>
          <CodeViewer content={content} contentType={contentType} maxHeight="32rem" />
        </ErrorBoundary>
      </div>
    </div>
  );
}
