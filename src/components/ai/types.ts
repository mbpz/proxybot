// ============================================================
// AI Panel Types
// ============================================================

import type { AlertSeverity } from "../../generated/desktop-contract";

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
  states: { id: string; label: string; state_type: string }[];
  transitions: {
    from_state: string;
    to_state: string;
    request_id: number;
    method: string;
    path: string;
    token_type: string | null;
    is_anomalous: boolean;
    anomaly_reason: string | null;
  }[];
  mermaid_md: string;
  anomalies: {
    request_id: number;
    anomaly_type: string;
    description: string;
    severity: AlertSeverity;
  }[];
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

export type SpecSource = "Llm" | "Heuristic" | "Hybrid";

export type SpecKind = "OpenApi" | "AsyncApi";

export interface SpecOutput {
  OpenApi?: string;
  AsyncApi?: string;
}

export interface CoverageReport {
  total_requests: number;
  covered_in_openapi: number;
  covered_in_asyncapi: number;
  uncovered_paths: string[];
  coverage_rate: number;
}

export interface ReplayFailure {
  path: string;
  method: string;
  expected_status: number;
  actual_status: number;
  body_diff_summary: string | null;
}

export interface ReplayReport {
  total: number;
  pass: number;
  fail: number;
  error: number;
  pass_rate: number;
  failures: ReplayFailure[];
  started_at: string;
  finished_at: string;
  mock_port: number;
}

export interface SpecResult {
  openapi: SpecOutput | null;
  asyncapi: SpecOutput | null;
  coverage: CoverageReport;
  replay: ReplayReport | null;
  generated_at: string;
  source: SpecSource;
  /**
   * Human-readable reason the LLM round was skipped or fell back
   * to the heuristic. Absent (`undefined`) when the LLM run was
   * clean. The panel renders this as a yellow banner above the
   * path list so users understand why the source badge says
   * `Heuristic` instead of `Llm`. The Rust side serialises this
   * with `skip_serializing_if = "Option::is_none"` so older
   * persisted specs deserialise back without the field.
   */
  degradation_reason?: string;
}

export type TrafficKind = "Http" | "WebSocket" | "Sse";

export interface TrafficRecord {
  method: string;
  path: string;
  host: string;
  request_body: string | null;
  response_status: number;
  response_body: string | null;
  timestamp: string;
  kind: TrafficKind;
}
