import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
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
    try {
      const result = await invoke<WsFrame[]>("get_ws_frames", { requestId });
      setFrames(result);
    } catch (err) {
      console.error("Failed to load WS frames:", err);
    }
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
          <div className="flex items-center justify-center h-full text-gray-500">
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
          <div className="flex items-center justify-center h-full text-gray-500">
            Select a frame to view details
          </div>
        )}
      </div>
    </div>
  );
}