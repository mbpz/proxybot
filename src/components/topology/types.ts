export type NodeKind = "device" | "app" | "host" | "proxy";

export interface TopologyNode {
  id: string;
  kind: NodeKind;
  label: string;
  app_tag: string | null;
  device_id: string | null;
  request_count: number;
  total_bytes: number;
  avg_latency_ms: number;
  error_count: number;
  error_rate: number;
  last_seen: number;
}

export interface TopologyEdge {
  id: string;
  from: string;
  to: string;
  request_count: number;
  total_bytes: number;
  avg_latency_ms: number;
  error_rate: number;
  is_anomalous: boolean;
}

export interface TopologyMeta {
  total_requests: number;
  total_bytes: number;
  device_count: number;
  app_count: number;
  host_count: number;
  time_range: [number, number];
  built_at: number;
}

export interface TopologyGraph {
  nodes: TopologyNode[];
  edges: TopologyEdge[];
  meta: TopologyMeta;
}

export type TimeWindow =
  | { type: "last_5_min" }
  | { type: "last_1_hour" }
  | { type: "session" }
  | { type: "custom"; start: number; end: number };

export interface TopologyFilter {
  device_ids?: string[] | null;
  app_tags?: string[] | null;
  host_contains?: string | null;
  time_window?: TimeWindow | null;
  sync_global: boolean;
}

export type ViewMode = "radial" | "layered" | "grouped";

export interface RecentRequest {
  id: string;
  method: string;
  host: string;
  path: string;
  status: number | null;
  duration_ms: number;
  timestamp: number;
}

export interface StatusCount {
  status_class: string;
  count: number;
}

export interface NodeDetail {
  node: TopologyNode;
  recent_requests: RecentRequest[];
  status_breakdown: StatusCount[];
}
