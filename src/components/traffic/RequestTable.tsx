import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";

interface InterceptedRequest {
  id: string;
  method: string;
  host: string;
  path: string;
  status?: number;
  duration_ms: number;
  timestamp: number;
  app_tag?: string;
}

interface RequestTableProps {
  requests: InterceptedRequest[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

function getStatusColor(status?: number): string {
  if (!status) return "text-gray-500";
  if (status >= 200 && status < 300) return "text-green-600";
  if (status >= 400 && status < 500) return "text-orange-600";
  if (status >= 500) return "text-red-600";
  return "text-gray-600";
}

function formatTime(timestamp: number): string {
  const d = new Date(timestamp * 1000);
  return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}`;
}

export function RequestTable({ requests, selectedId, onSelect }: RequestTableProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const rowVirtualizer = useVirtualizer({
    count: requests.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 48,
  });

  if (requests.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No requests captured yet
      </div>
    );
  }

  return (
    <div ref={parentRef} className="h-full overflow-auto">
      <div
        style={{
          height: `${rowVirtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const req = requests[virtualRow.index];
          return (
            <div
              key={req.id}
              onClick={() => onSelect(req.id)}
              className={`absolute top-0 left-0 w-full flex items-center px-4 border-b cursor-pointer hover:bg-gray-50 ${
                req.id === selectedId ? "bg-blue-100" : ""
              }`}
              style={{
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              <span className="w-16 text-sm font-mono">{req.method}</span>
              <span className="flex-1 truncate text-sm">{req.path}</span>
              <span className={`w-16 text-sm ${getStatusColor(req.status)}`}>
                {req.status || ".."}
              </span>
              <span className="w-20 text-right text-sm text-gray-500">
                {req.duration_ms}ms
              </span>
              <span className="w-20 text-right text-xs text-gray-400">
                {formatTime(req.timestamp)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
