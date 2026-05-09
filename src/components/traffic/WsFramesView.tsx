import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface WsFrame {
  id: string;
  direction: "incoming" | "outgoing";
  opcode: number;
  payload: string;
  timestamp: number;
}

interface WsFramesViewProps {
  requestId: string;
}

function getOpcodeName(opcode: number): string {
  switch (opcode) {
    case 0x01:
      return "TEXT";
    case 0x02:
      return "BINARY";
    case 0x08:
      return "CLOSE";
    case 0x09:
      return "PING";
    case 0x0a:
      return "PONG";
    default:
      return `OP${opcode}`;
  }
}

export function WsFramesView({ requestId }: WsFramesViewProps) {
  const [frames, setFrames] = useState<WsFrame[]>([]);
  const [selectedFrame, setSelectedFrame] = useState<WsFrame | null>(null);

  useEffect(() => {
    loadFrames();
  }, [requestId]);

  async function loadFrames() {
    try {
      const result = await invoke<WsFrame[]>("get_ws_frames", { requestId });
      setFrames(result);
    } catch (err) {
      console.error("Failed to load WS frames:", err);
    }
  }

  return (
    <div className="flex h-full">
      <div className="w-1/2 border-r overflow-auto">
        {frames.length === 0 ? (
          <div className="p-4 text-gray-500">No WebSocket frames</div>
        ) : (
          frames.map((frame) => (
            <div
              key={frame.id}
              onClick={() => setSelectedFrame(frame)}
              className={`flex items-center px-3 py-2 border-b cursor-pointer ${
                frame.direction === "incoming" ? "text-green-600" : "text-blue-600"
              } ${selectedFrame?.id === frame.id ? "bg-gray-100" : ""}`}
            >
              <span className="w-4">{frame.direction === "incoming" ? "←" : "→"}</span>
              <span className="w-12 font-mono text-xs">{getOpcodeName(frame.opcode)}</span>
              <span className="flex-1 truncate text-sm">{frame.payload.slice(0, 30)}</span>
            </div>
          ))
        )}
      </div>
      <div className="w-1/2 p-4 overflow-auto">
        {selectedFrame ? (
          <pre className="text-sm font-mono whitespace-pre-wrap">{selectedFrame.payload}</pre>
        ) : (
          <div className="text-gray-500">Select a frame to view</div>
        )}
      </div>
    </div>
  );
}