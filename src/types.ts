export type {
  BreakpointTarget,
  InterceptedRequest,
  Rule,
  RuleAction,
  RulePattern,
  WsFrame,
} from "./generated/desktop-contract";

export type AppTab = "all" | "WeChat" | "Douyin" | "Alipay" | "Unknown";

export interface NetworkInfo {
  lan_ip: string;
  interface: string;
}

export interface DnsEntry {
  domain: string;
  timestamp_ms: number;
  app_name?: string;
  app_icon?: string;
}

export interface CaMetadata {
  created_at: number;
  serial: string;
}

export interface DeviceInfo {
  id: number;
  mac_address: string;
  name: string;
  created_at: string;
  last_seen_at: string;
  upload_bytes: number;
  download_bytes: number;
  rule_override: string | null;
}

export interface ReplayTarget {
  host: string;
  request_count: number;
  path_count: number;
}

export interface ReplayResult {
  request_id: number;
  method: string;
  url: string;
  recorded_response: RecordedResponse;
  mock_response: MockResponse | null;
  diff: DiffResult | null;
  delay_ms: number;
  error: string | null;
}

export interface RecordedResponse {
  status: number;
  headers: [string, string][];
  body: string | null;
}

export interface MockResponse {
  status: number;
  headers: [string, string][];
  body: string | null;
}

export interface DiffResult {
  header_diffs: HeaderDiff[];
  body_diff: BodyDiff | null;
  has_changes: boolean;
}

export interface HeaderDiff {
  header: string;
  recorded: string | null;
  mock: string | null;
  diff_type: "Added" | "Removed" | "Modified" | "Unchanged";
}

export interface BodyDiff {
  recorded: string | null;
  mock: string | null;
  recorded_lines: string[];
  mock_lines: string[];
  line_diffs: LineDiff[];
}

export interface LineDiff {
  line_number_recorded: number | null;
  line_number_mock: number | null;
  recorded_text: string | null;
  mock_text: string | null;
  diff_type: "Added" | "Removed" | "Modified" | "Unchanged";
}

export interface Alert {
  id: number;
  device_id: number | null;
  severity: "Info" | "Warning" | "Critical";
  alert_type: string;
  details: string;
  created_at: string;
  acknowledged: boolean;
}

export interface AuthState {
  id: string;
  label: string;
  state_type: "Initial" | "Login" | "Authenticated" | "Resource" | "Logout" | "Error";
}

export interface AuthTransition {
  from_state: string;
  to_state: string;
  request_id: number;
  method: string;
  path: string;
  token_type: string | null;
  is_anomalous: boolean;
  anomaly_reason: string | null;
}

export interface AuthStateMachine {
  device_id: number | null;
  states: AuthState[];
  transitions: AuthTransition[];
  mermaid_md: string;
  anomalies: Anomaly[];
}

export interface Anomaly {
  request_id: number;
  anomaly_type: string;
  description: string;
  severity: "Info" | "Warning" | "Critical";
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
  text: string | null;
  position: VisionPosition;
  children: VisionComponent[];
}

export interface VisionPosition {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ComponentTree {
  components: VisionComponent[];
  layout_json: string;
  suggested_routes: string[];
}
