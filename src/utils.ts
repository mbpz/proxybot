export function formatTimestamp(ts: string): string {
  try {
    const parts = ts.split(".");
    const secs = parseInt(parts[0]);
    if (isNaN(secs)) return ts;
    const ms = parts[1] || "000";
    const date = new Date(secs * 1000);
    if (isNaN(date.getTime())) return ts;
    return date.toLocaleTimeString() + "." + ms.slice(0, 3);
  } catch {
    return ts;
  }
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
}

export function formatBody(body: string | undefined, headers: [string, string][]): string {
  if (!body) return "";
  const contentType = headers.find(([name]) => name.toLowerCase() === "content-type");
  if (contentType && contentType[1].includes("application/json")) {
    try {
      return JSON.stringify(JSON.parse(body), null, 2);
    } catch {
      return body;
    }
  }
  return body;
}

export function appBadgeClass(appName: string | undefined): string {
  if (!appName) return "badge-unknown";
  const lower = appName.toLowerCase();
  if (lower.includes("wechat")) return "badge-wechat";
  if (lower.includes("douyin")) return "badge-douyin";
  if (lower.includes("alipay")) return "badge-alipay";
  return "badge-unknown";
}

export function getStatusColor(status?: number): string {
  if (!status) return "var(--text-muted)";
  if (status >= 200 && status < 300) return "var(--accent-green)";
  if (status >= 300 && status < 400) return "var(--accent-blue)";
  if (status >= 400 && status < 500) return "var(--accent-yellow)";
  if (status >= 500) return "var(--accent-red)";
  return "var(--text-secondary)";
}

export function getStatusTailwindClass(status?: number): string {
  if (!status) return "text-text-muted";
  if (status >= 200 && status < 300) return "text-accent-green";
  if (status >= 300 && status < 400) return "text-accent-blue";
  if (status >= 400 && status < 500) return "text-accent-yellow";
  if (status >= 500) return "text-accent-red";
  return "text-text-secondary";
}
