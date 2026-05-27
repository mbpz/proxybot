import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { MethodBadge } from "../ui/Badge";

interface InterceptedRequest {
  id: string;
  method: string;
  host: string;
  path: string;
  status?: number;
  duration_ms: number;
  timestamp: number;
  app_tag?: string;
  size?: number;
}

interface RequestTableProps {
  requests: InterceptedRequest[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

function getStatusColor(status?: number): string {
  if (!status) return "var(--text-muted)";
  if (status >= 200 && status < 300) return "var(--accent-green)";
  if (status >= 300 && status < 400) return "var(--accent-blue)";
  if (status >= 400 && status < 500) return "var(--accent-yellow)";
  if (status >= 500) return "var(--accent-red)";
  return "var(--text-secondary)";
}

function formatTime(timestamp: number): string {
  const d = new Date(timestamp * 1000);
  return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}`;
}

function formatSize(bytes?: number): string {
  if (!bytes) return "-";
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

export function RequestTable({ requests, selectedId, onSelect }: RequestTableProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const rowVirtualizer = useVirtualizer({
    count: requests.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 40,
  });

  if (requests.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">📡</div>
        <div className="empty-state-title">No requests captured yet</div>
        <div className="empty-state-description">
          Start the proxy and configure your device to see traffic here
        </div>
      </div>
    );
  }

  return (
    <div ref={parentRef} className="h-full overflow-auto">
      {/* Header */}
      <div
        className="flex items-center px-3 py-2 text-xs font-mono uppercase"
        style={{
          background: "var(--bg-tertiary)",
          borderBottom: "1px solid var(--border)",
          color: "var(--text-secondary)",
          position: "sticky",
          top: 0,
          zIndex: 1,
        }}
      >
        <span className="w-20">Method</span>
        <span className="flex-1">Host / Path</span>
        <span className="w-16 text-center">Status</span>
        <span className="w-16 text-center">Size</span>
        <span className="w-20 text-right">Time</span>
      </div>

      {/* Virtual rows */}
      <div
        style={{
          height: `${rowVirtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const req = requests[virtualRow.index];
          const isSelected = req.id === selectedId;

          return (
            <div
              key={req.id}
              onClick={() => onSelect(req.id)}
              className="absolute top-0 left-0 w-full flex items-center px-3 cursor-pointer"
              style={{
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
                background: isSelected ? "var(--bg-tertiary)" : "transparent",
                borderBottom: "1px solid var(--border)",
                transition: "background var(--transition-fast)",
              }}
              onMouseEnter={(e) => {
                if (!isSelected) {
                  e.currentTarget.style.background = "var(--bg-elevated)";
                }
              }}
              onMouseLeave={(e) => {
                if (!isSelected) {
                  e.currentTarget.style.background = "transparent";
                }
              }}
            >
              <span className="w-20">
                <MethodBadge method={req.method} />
              </span>
              <span className="flex-1 truncate text-sm">
                <span className="font-mono text-secondary">{req.host}</span>
                <span className="text-muted">{req.path}</span>
              </span>
              <span
                className="w-16 text-center text-sm font-mono"
                style={{ color: getStatusColor(req.status) }}
              >
                {req.status || ".."}
              </span>
              <span className="w-16 text-center text-xs text-muted font-mono">
                {formatSize(req.size)}
              </span>
              <span className="w-20 text-right text-xs text-muted font-mono">
                {formatTime(req.timestamp)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
