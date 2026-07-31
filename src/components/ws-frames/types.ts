// The desktop wire shape is generated from Rust; this module adds display logic only.
export type { WsFrame, WsFrameEvent } from "../../generated/desktop-contract";

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
