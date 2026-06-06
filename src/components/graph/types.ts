export interface RequestNode {
  id: string;
  host: string;
  path: string;
  method: string;
  status?: number;
  duration_ms: number;
  timestamp: number;
  parentId?: string;
}

export interface Edge {
  from: string;
  to: string;
}

export interface GraphData {
  requests: RequestNode[];
  edges: Edge[];
}
