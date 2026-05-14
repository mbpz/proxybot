import { useUpdateCheck } from "../../hooks/useUpdateCheck";

export function UpdateSettings() {
  const { hasUpdate, latestVersion, currentVersion, isLoading, error, checkForUpdates, openReleasePage } = useUpdateCheck();

  return (
    <div className="border-t pt-4 mt-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium">检查更新</span>
          {isLoading && (
            <span className="text-xs text-gray-400">检查中...</span>
          )}
          {error && (
            <span className="text-xs text-red-500">{error}</span>
          )}
          {!isLoading && !error && hasUpdate && (
            <span className="text-xs bg-red-500 text-white px-2 py-0.5 rounded">NEW</span>
          )}
        </div>

        <div className="flex items-center gap-3">
          {!isLoading && !error && hasUpdate && latestVersion && (
            <span className="text-xs text-gray-400">v{latestVersion} 可用</span>
          )}
          {!isLoading && !error && !hasUpdate && (
            <span className="text-xs text-gray-400">v{currentVersion}</span>
          )}
          <button
            onClick={hasUpdate ? openReleasePage : checkForUpdates}
            disabled={isLoading}
            className={`px-3 py-1 text-xs rounded ${
              hasUpdate
                ? "bg-purple-600 text-white hover:bg-purple-700"
                : "bg-gray-200 text-gray-700 hover:bg-gray-300"
            }`}
          >
            {hasUpdate ? "更新 ProxyBot" : "检查更新"}
          </button>
        </div>
      </div>
    </div>
  );
}