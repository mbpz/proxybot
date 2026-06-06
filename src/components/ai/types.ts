// ============================================================
// AI Panel Types
// ============================================================

export interface AiStatsData {
  stats: AiStatRow[];
}

export interface AiStatRow {
  provider: string;
  model: string;
  total_tokens: number;
  prompt_tokens: number;
  completion_tokens: number;
  cost_usd: number;
  requests: number;
}

export interface InferredApi {
  id: number;
  session_id: string;
  name: string;
  method: string;
  path: string;
  params: string;
  auth_required: boolean;
  score: number | null;
  created_at: string;
}

export interface AuthStateMachine {
  device_id: number | null;
  states: { name: string; is_initial: boolean; is_terminal: boolean }[];
  transitions: { from: string; to: string; label: string }[];
  mermaid_md: string;
  anomalies: { description: string; severity: string }[];
}

export interface VisionAnalysis {
  id: number;
  session_id: string;
  filename: string;
  components: VisionComponent[];
  raw_response: string;
  score: number;
  created_at: string;
}

export interface VisionComponent {
  component_type: string;
  text?: string;
  position: { x: number; y: number; width: number; height: number };
  children: VisionComponent[];
}
