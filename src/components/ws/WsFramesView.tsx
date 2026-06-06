import { useState, useEffect, useCallback } from "react";
import { safeInvokeOr } from "../../utils/safeInvoke";
import { listen } from "@tauri-apps/api/event";
import { WsFrameItem } from "./WsFrameItem";
import { WsFrameDetail } from "./WsFrameDetail";

interface WsFrame {
  id: string;
  requestId?: string;
  direction: "incoming" | "outgoing";
  opcode: number;
  payload: string;
  timestamp: number;
}

interface WsFramesViewProps {
  requestId: string;
}

export function WsFramesView({ requestId }: WsFramesViewProps) {
  const [frames, setFrames] = useState<WsFrame[]>([]);
  const [selectedFrame, setSelectedFrame] = useState<WsFrame | null>(null);

  const loadFrames = useCallback(async () => {
    const result = await safeInvokeOr<WsFrame[]>("get_ws_frames", [], { requestId });
    setFrames(result);
  }, [requestId]);

  useEffect(() => {
    loadFrames();

    // Subscribe to real-time frames
    const unlisten = listen<WsFrame>("ws_frame", (event) => {
      if (event.payload.requestId === requestId) {
        setFrames((prev) => [...prev, event.payload]);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [requestId, loadFrames]);

  return (
    <div className="flex h-full">
      {/* Frame List */}
      <div className="w-1/2 border-r overflow-auto">
        {frames.length === 0 ? (
          <div className="flex items-center justify-center h-full text-text-muted">
            No WebSocket frames captured
          </div>
        ) : (
          frames.map((frame) => (
            <WsFrameItem
              key={frame.id}
              frame={frame}
              isSelected={selectedFrame?.id === frame.id}
              onClick={() => setSelectedFrame(frame)}
            />
          ))
        )}
      </div>

      {/* Frame Detail */}
      <div className="w-1/2 overflow-hidden">
        {selectedFrame ? (
          <WsFrameDetail frame={selectedFrame} />
        ) : (
          <div className="flex items-center justify-center h-full text-text-muted">
            Select a frame to view details
          </div>
        )}
      </div>
    </div>
  );
}