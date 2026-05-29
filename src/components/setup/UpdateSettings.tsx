import { useUpdateCheck } from "../../hooks/useUpdateCheck";

export function UpdateSettings() {
  const { hasUpdate, latestVersion, currentVersion, isLoading, error, checkForUpdates, openReleasePage } = useUpdateCheck();

  return (
    <div className="border-t border-border pt-4 mt-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium">检查更新</span>
          {isLoading && (
            <span className="text-xs text-text-muted">检查中...</span>
          )}
          {error && (
            <span className="text-xs text-accent-red">{error}</span>
          )}
          {!isLoading && !error && hasUpdate && (
            <span className="text-xs bg-accent-red text-white px-2 py-0.5 rounded">NEW</span>
          )}
        </div>

        <div className="flex items-center gap-3">
          {!isLoading && !error && hasUpdate && latestVersion && (
            <span className="text-xs text-text-muted">v{latestVersion} 可用</span>
          )}
          {!isLoading && !error && !hasUpdate && (
            <span className="text-xs text-text-muted">v{currentVersion}</span>
          )}
          <button
            onClick={hasUpdate ? openReleasePage : checkForUpdates}
            disabled={isLoading}
            className={`btn btn-sm ${hasUpdate ? "btn-primary" : "btn-secondary"}`}
          >
            {hasUpdate ? "更新 ProxyBot" : "检查更新"}
          </button>
        </div>
      </div>
    </div>
  );
}
