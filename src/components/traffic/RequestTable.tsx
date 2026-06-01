import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Radio } from "lucide-react";

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

const APP_COLORS = {
  WeChat: 'var(--accent-green)',
  Douyin: 'var(--accent-purple)',
  Alipay: 'var(--accent-blue)',
  Blocked: 'var(--accent-red)',
} as const;

function formatSize(bytes?: number): string {
  if (!bytes) return "-";
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

function getAppColor(appTag: string): string {
  return APP_COLORS[appTag as keyof typeof APP_COLORS] || 'var(--text-secondary)';
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
      <div className="empty-state">
        <Radio size={48} className="text-text-muted mb-4 opacity-50" />
        <div className="empty-state-title">No requests captured yet</div>
        <div className="empty-state-description">
          Start the proxy and configure your device to see traffic here
        </div>
      </div>
    );
  }

  return (
    <div ref={parentRef} className="h-full overflow-auto bg-bg-primary">
      {/* Traffic Rows Container */}
      <div className="flex flex-col gap-1 p-3" style={{ gap: 4 }}>
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const req = requests[virtualRow.index];
          const isSelected = req.id === selectedId;

          return (
            <div
              key={req.id}
              onClick={() => onSelect(req.id)}
              className="flex items-center cursor-pointer"
              style={{
                height: `${virtualRow.size}px`,
                padding: '10px 12px',
                gap: 12,
                background: isSelected ? '#00d4ff14' : 'transparent',
                borderLeft: isSelected ? '2px solid #00d4ff' : '2px solid transparent',
                fontFamily: 'Inter',
              }}
            >
              <span className="flex-1 truncate" style={{ fontSize: 13, color: '#fff', fontFamily: 'Inter' }}>
                {req.host}
              </span>
              <span className="w-24 flex justify-center" style={{ fontFamily: 'Inter', fontSize: 11 }}>
                {req.app_tag && (
                  <span
                    style={{
                      fontFamily: 'Inter',
                      fontSize: 11,
                      color: getAppColor(req.app_tag),
                    }}
                  >
                    {req.app_tag}
                  </span>
                )}
              </span>
              <span className="w-20 text-right" style={{ fontSize: 11, color: '#8888aa', fontFamily: 'Inter' }}>
                {formatSize(req.size)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
