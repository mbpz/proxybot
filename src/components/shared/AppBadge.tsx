interface AppBadgeProps {
  appId: string;
  appName: string;
  confidence?: number;
}

const appColors: Record<string, string> = {
  tiktok: "bg-pink-100 text-pink-800",
  wechat: "bg-green-100 text-green-800",
  douyin: "bg-blue-100 text-blue-800",
  alipay: "bg-indigo-100 text-indigo-800",
  amazon: "bg-orange-100 text-orange-800",
  apple: "bg-gray-100 text-gray-800",
};

export function AppBadge({ appId, appName, confidence }: AppBadgeProps) {
  const colorClass = appColors[appId] || "bg-gray-100 text-gray-600";

  return (
    <span
      className={`px-2 py-1 rounded text-xs font-medium ${colorClass}`}
      title={confidence != null ? `${appName} (${(confidence * 100).toFixed(0)}% confidence)` : appName}
    >
      {appName}
    </span>
  );
}
