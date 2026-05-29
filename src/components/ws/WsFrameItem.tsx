interface WsFrame {
  id: string;
  direction: "incoming" | "outgoing";
  opcode: number;
  payload: string;
  timestamp: number;
}

interface WsFrameItemProps {
  frame: WsFrame;
  isSelected: boolean;
  onClick: () => void;
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

function formatTime(timestamp: number): string {
  const d = new Date(timestamp * 1000);
  return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}.${d.getMilliseconds().toString().padStart(3, "0")}`;
}

function truncate(str: string, maxLen: number): string {
  if (str.length <= maxLen) return str;
  return str.slice(0, maxLen) + "...";
}

export function WsFrameItem({ frame, isSelected, onClick }: WsFrameItemProps) {
  return (
    <div
      onClick={onClick}
      className={`flex items-center px-3 py-2 border-b border-border cursor-pointer hover:bg-surface-elevated ${
        isSelected ? "bg-surface-tertiary" : ""
      } ${frame.direction === "incoming" ? "text-accent-green" : "text-accent-blue"}`}
    >
      <span className="w-4 text-lg">
        {frame.direction === "incoming" ? "←" : "→"}
      </span>
      <span className="w-12 font-mono text-xs">{getOpcodeName(frame.opcode)}</span>
      <span className="flex-1 truncate text-sm">{truncate(frame.payload, 40)}</span>
      <span className="text-xs text-text-muted ml-2">{formatTime(frame.timestamp)}</span>
    </div>
  );
}