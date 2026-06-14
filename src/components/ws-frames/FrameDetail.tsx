import { useState } from "react";
import { WsFrame, getOpcodeName } from "./types";
import { HexDump } from "./HexDump";

interface FrameDetailProps {
  frame: WsFrame;
}

function formatTimestamp(ts: string): string {
  const d = new Date(ts);
  return d.toLocaleTimeString();
}

export function FrameDetail({ frame }: FrameDetailProps) {
  const [viewMode, setViewMode] = useState<"text" | "hex">("text");

  return (
    <div className="space-y-4 p-4">
      {/* Metadata grid */}
      <div className="grid grid-cols-2 gap-2 text-sm">
        <div>
          <span className="text-text-muted">Direction: </span>
          <span
            className={
              frame.direction === "incoming" ? "text-accent-green" : "text-accent-blue"
            }
          >
            {frame.direction === "incoming" ? "← incoming" : "→ outgoing"}
          </span>
        </div>
        <div>
          <span className="text-text-muted">Opcode: </span>
          {frame.opcode} ({getOpcodeName(frame.opcode)})
        </div>
        <div>
          <span className="text-text-muted">Size: </span>
          {frame.size} bytes
        </div>
        <div>
          <span className="text-text-muted">Time: </span>
          {formatTimestamp(frame.timestamp)}
        </div>
      </div>

      {/* Text/Hex toggle */}
      <div className="flex gap-2">
        <button
          onClick={() => setViewMode("text")}
          className={`text-xs px-3 py-1 rounded ${
            viewMode === "text" ? "bg-accent-blue text-white" : "bg-surface-tertiary"
          }`}
        >
          Text
        </button>
        <button
          onClick={() => setViewMode("hex")}
          className={`text-xs px-3 py-1 rounded ${
            viewMode === "hex" ? "bg-accent-blue text-white" : "bg-surface-tertiary"
          }`}
        >
          Hex
        </button>
      </div>

      {/* Payload */}
      {viewMode === "text" ? (
        <pre className="bg-surface-tertiary rounded p-3 font-mono text-xs overflow-auto whitespace-pre-wrap break-all">
          {frame.payload || "(empty)"}
        </pre>
      ) : (
        <HexDump payload={frame.payload} truncated={frame.truncated} />
      )}
    </div>
  );
}
