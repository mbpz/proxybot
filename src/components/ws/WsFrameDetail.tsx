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
      <div className="p-4 border-b bg-gray-50">
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div>
            <span className="text-gray-500">Direction:</span>{" "}
            <span className={frame.direction === "incoming" ? "text-green-600" : "text-blue-600"}>
              {frame.direction === "incoming" ? "Incoming ←" : "Outgoing →"}
            </span>
          </div>
          <div>
            <span className="text-gray-500">Opcode:</span>{" "}
            <span className="font-mono">{frame.opcode} ({getOpcodeName(frame.opcode)})</span>
          </div>
          <div>
            <span className="text-gray-500">Size:</span>{" "}
            <span>{frame.payload.length} bytes</span>
          </div>
          <div>
            <span className="text-gray-500">Time:</span>{" "}
            <span>{new Date(frame.timestamp * 1000).toLocaleString()}</span>
          </div>
        </div>
      </div>

      {/* View Mode Toggle */}
      <div className="flex gap-2 p-2 border-b">
        <button
          onClick={() => setViewMode("text")}
          className={`px-3 py-1 rounded text-sm ${
            viewMode === "text" ? "bg-blue-500 text-white" : "bg-gray-200"
          }`}
        >
          Text
        </button>
        <button
          onClick={() => setViewMode("hex")}
          className={`px-3 py-1 rounded text-sm ${
            viewMode === "hex" ? "bg-blue-500 text-white" : "bg-gray-200"
          }`}
        >
          Hex
        </button>
      </div>

      {/* Payload */}
      <div className="flex-1 overflow-auto p-4 bg-gray-100">
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
