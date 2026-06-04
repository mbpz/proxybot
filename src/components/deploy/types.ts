export interface DeploymentBundle {
  name: string;
  base_path: string;
  mock_api_path: string;
  frontend_path: string;
  docker_compose_content: string;
  readme_content: string;
  ci_template_content: string;
}

export interface DeploymentResult {
  success: boolean;
  bundle_path: string;
  message: string;
}

export interface DeploymentRecord {
  id: number;
  session_id: string;
  project_name: string;
  bundle_path: string;
  last_git_init_at: string | null;
  created_at: string;
  updated_at: string;
}

export type DeployTab = "compose" | "readme" | "ci";
