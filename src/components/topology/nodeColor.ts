import { TopologyNode } from "./types";

const COLORS = {
  device: { background: "rgba(0,212,255,0.15)", border: "#00d4ff" },
  app: { background: "rgba(34,197,94,0.12)", border: "#22c55e" },
  host: { background: "rgba(136,136,170,0.1)", border: "#1e1e2e" },
  proxy: { background: "rgba(168,85,247,0.15)", border: "#a855f7" },
};

const ERROR_BORDER = "#ff4d4d";
const ERROR_THRESHOLD = 0.10;

export function nodeBackgroundColor(node: TopologyNode): string {
  return COLORS[node.kind].background;
}

export function nodeBorderColor(node: TopologyNode): string {
  if (node.error_rate >= ERROR_THRESHOLD) return ERROR_BORDER;
  return COLORS[node.kind].border;
}

export function nodeSize(node: TopologyNode, baseSize = 18, scale = 4): number {
  return baseSize + Math.log2(Math.max(node.request_count, 1)) * scale;
}
