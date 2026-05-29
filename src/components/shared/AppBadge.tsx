interface AppBadgeProps {
  appId: string;
  appName: string;
  confidence?: number;
}

const appColors: Record<string, string> = {
  tiktok: "badge-douyin",
  wechat: "badge-wechat",
  douyin: "badge-douyin",
  alipay: "badge-alipay",
  amazon: "badge-warning",
  apple: "badge-unknown",
};

export function AppBadge({ appId, appName, confidence }: AppBadgeProps) {
  const colorClass = appColors[appId] || "badge-unknown";

  return (
    <span
      className={`badge ${colorClass}`}
      title={confidence != null ? `${appName} (${(confidence * 100).toFixed(0)}% confidence)` : appName}
    >
      {appName}
    </span>
  );
}
