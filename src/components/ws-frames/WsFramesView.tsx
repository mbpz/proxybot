import { useEffect, useState } from "react";
import { desktop } from "../../desktop/contract";
import { getOpcodeName } from "./types";
import type { WsFrame } from "./types";
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
    let disposed = false;
    let initialLoaded = false;
    const buffered: WsFrame[] = [];
    const subscription = desktop.subscribe("ws-frame:new", {
      next: (event) => {
        if (event.request_id !== requestId) return;
        if (!initialLoaded) buffered.push(event.frame);
        else setFrames((current) => [...current, event.frame]);
      },
      error: (error) => console.error("Invalid WebSocket frame event:", error),
    });

    void (async () => {
      try {
        await subscription.ready;
        const initial = await desktop.call("get_ws_frames", { requestId });
        initialLoaded = true;
        if (!disposed) setFrames([...initial, ...buffered]);
      } catch (error) {
        console.error("WebSocket frame setup failed:", error);
      }
    })();

    return () => {
      disposed = true;
      subscription.dispose();
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
