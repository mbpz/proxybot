import { useState } from "react";
import { HexDump } from "./HexDump";

interface WsFrame {
  id: string;
  direction: "incoming" | "outgoing";
  opcode: number;
  payload: string;
  timestamp: number;
}

interface WsFrameDetailProps {
  frame: WsFrame;
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

export function WsFrameDetail({ frame }: WsFrameDetailProps) {
  const [viewMode, setViewMode] = useState<"text" | "hex">("text");

  return (
    <div className="h-full flex flex-col">
      {/* Metadata */}
      <div className="p-4 border-b border-border bg-surface-tertiary">
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div>
            <span className="text-text-muted">Direction:</span>{" "}
            <span className={frame.direction === "incoming" ? "text-accent-green" : "text-accent-blue"}>
              {frame.direction === "incoming" ? "Incoming ←" : "Outgoing →"}
            </span>
          </div>
          <div>
            <span className="text-text-muted">Opcode:</span>{" "}
            <span className="font-mono">{frame.opcode} ({getOpcodeName(frame.opcode)})</span>
          </div>
          <div>
            <span className="text-text-muted">Size:</span>{" "}
            <span>{frame.payload.length} bytes</span>
          </div>
          <div>
            <span className="text-text-muted">Time:</span>{" "}
            <span>{new Date(frame.timestamp * 1000).toLocaleString()}</span>
          </div>
        </div>
      </div>

      {/* View Mode Toggle */}
      <div className="flex gap-2 p-2 border-b">
        <button
          onClick={() => setViewMode("text")}
          className={`px-3 py-1 rounded text-sm ${
            viewMode === "text" ? "bg-accent-blue text-white" : "bg-surface-tertiary"
          }`}
        >
          Text
        </button>
        <button
          onClick={() => setViewMode("hex")}
          className={`px-3 py-1 rounded text-sm ${
            viewMode === "hex" ? "bg-accent-blue text-white" : "bg-surface-tertiary"
          }`}
        >
          Hex
        </button>
      </div>

      {/* Payload */}
      <div className="flex-1 overflow-auto p-4 bg-surface-primary">
        {viewMode === "text" ? (
          <pre className="text-sm font-mono whitespace-pre-wrap break-all">
            {frame.payload}
          </pre>
        ) : (
          <HexDump text={frame.payload} />
        )}
      </div>
    </div>
  );
}
