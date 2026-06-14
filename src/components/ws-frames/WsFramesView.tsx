import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { WsFrame, WsFrameEvent, getOpcodeName } from "./types";
import { FrameDetail } from "./FrameDetail";

interface WsFramesViewProps {
  requestId: string;
}

function truncate(s: string, n: number): string {
  if (s.length <= n) return s;
  return s.slice(0, n) + "…";
}

export function WsFramesView({ requestId }: WsFramesViewProps) {
  const [frames, setFrames] = useState<WsFrame[]>([]);
  const [selectedFrame, setSelectedFrame] = useState<WsFrame | null>(null);

  useEffect(() => {
    let unlistenFn: (() => void) | null = null;

    // Initial fetch
    invoke<WsFrame[]>("get_ws_frames", { requestId })
      .then((initial) => setFrames(initial))
      .catch(console.error);

    // Subscribe to real-time updates
    listen<WsFrameEvent>("ws-frame:new", (event) => {
      if (event.payload.request_id === requestId) {
        setFrames((prev) => [...prev, event.payload.frame]);
      }
    }).then((fn) => {
      unlistenFn = fn;
    });

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [requestId]);

  return (
    <div className="flex h-full">
      {/* Frame list (left half) */}
      <div className="w-1/2 border-r overflow-auto" data-testid="ws-frames-list">
        {frames.length === 0 ? (
          <p className="p-4 text-sm text-text-muted">
            No WebSocket frames for this request.
          </p>
        ) : (
          frames.map((frame, i) => (
            <div
              key={i}
              onClick={() => setSelectedFrame(frame)}
              data-testid="ws-frame-row"
              className={`flex items-center px-3 py-2 border-b border-border cursor-pointer ${
                selectedFrame === frame ? "bg-surface-elevated" : "hover:bg-surface-tertiary"
              }`}
            >
              <span className="w-4 text-sm">
                {frame.direction === "incoming" ? "←" : "→"}
              </span>
              <span className="w-12 font-mono text-xs text-text-muted">
                {getOpcodeName(frame.opcode)}
              </span>
              <span className="flex-1 truncate text-sm font-mono">
                {truncate(frame.payload, 30)}
                {frame.truncated && (
                  <span className="ml-1 text-xs text-accent-yellow">(truncated)</span>
                )}
              </span>
              <span className="text-xs text-text-muted">
                {new Date(frame.timestamp).toLocaleTimeString()}
              </span>
            </div>
          ))
        )}
      </div>

      {/* Frame detail (right half) */}
      <div className="w-1/2 overflow-auto">
        {selectedFrame ? (
          <FrameDetail frame={selectedFrame} />
        ) : (
          <p className="p-4 text-sm text-text-muted">
            Select a frame to view details.
          </p>
        )}
      </div>
    </div>
  );
}
