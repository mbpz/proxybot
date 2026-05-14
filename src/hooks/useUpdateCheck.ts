import { useState, useEffect, useCallback } from "react";

interface UpdateInfo {
  hasUpdate: boolean;
  latestVersion: string | null;
  currentVersion: string;
  releaseUrl: string | null;
  isLoading: boolean;
  error: string | null;
}

const CURRENT_VERSION = "1.2.0";
const REPO_OWNER = "mbpz";
const REPO_NAME = "proxybot";

export function useUpdateCheck() {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo>({
    hasUpdate: false,
    latestVersion: null,
    currentVersion: CURRENT_VERSION,
    releaseUrl: null,
    isLoading: false,
    error: null,
  });

  const checkForUpdates = useCallback(async () => {
    setUpdateInfo(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      const response = await fetch(
        `https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest`
      );

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json();
      const latestVersion = data.tag_name?.replace(/^v/, "") || "";
      const hasUpdate = compareVersions(latestVersion, CURRENT_VERSION) > 0;

      setUpdateInfo({
        hasUpdate,
        latestVersion,
        currentVersion: CURRENT_VERSION,
        releaseUrl: data.html_url || null,
        isLoading: false,
        error: null,
      });
    } catch (err) {
      setUpdateInfo(prev => ({
        ...prev,
        isLoading: false,
        error: err instanceof Error ? err.message : "检查更新失败",
      }));
    }
  }, []);

  const openReleasePage = useCallback(() => {
    if (updateInfo.releaseUrl) {
      window.open(updateInfo.releaseUrl, "_blank");
    }
  }, [updateInfo.releaseUrl]);

  return { ...updateInfo, checkForUpdates, openReleasePage };
}

function compareVersions(latest: string, current: string): number {
  const la = latest.split(".").map(Number);
  const ca = current.split(".").map(Number);

  for (let i = 0; i < Math.max(la.length, ca.length); i++) {
    const l = la[i] || 0;
    const c = ca[i] || 0;
    if (l > c) return 1;
    if (l < c) return -1;
  }
  return 0;
}