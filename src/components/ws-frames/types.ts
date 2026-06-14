// Shared types for the WS Frame Viewer components.

export interface WsFrame {
  direction: "incoming" | "outgoing";
  timestamp: string;
  payload: string;
  size: number;
  opcode: number;
  truncated: boolean;
}

export interface WsFrameEvent {
  request_id: string;
  frame: WsFrame;
}

export function getOpcodeName(opcode: number): string {
  switch (opcode) {
    case 0x01:
      return "Text";
    case 0x02:
      return "Binary";
    case 0x08:
      return "Close";
    case 0x09:
      return "Ping";
    case 0x0a:
      return "Pong";
    default:
      return "Unknown";
  }
}
