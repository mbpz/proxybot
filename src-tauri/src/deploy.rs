//! Deployment bundle generator for ProxyBot.
//!
//! Produces a complete Docker Compose deployment with mock API, frontend, and postgres.
//! Initializes a git repo and sets up GitHub Actions CI for Playwright E2E tests.

use crate::db::DbState;
use crate::infer::InferredApi;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentBundle {
    pub name: String,
    pub base_path: String,
    pub mock_api_path: String,
    pub frontend_path: String,
    pub docker_compose_content: String,
    pub readme_content: String,
    pub ci_template_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentResult {
    pub success: bool,
    pub bundle_path: String,
    pub message: String,
}

// ============================================================================
// GitHub Actions CI Template
// ============================================================================

fn generate_github_actions_ci() -> String {
    r#"name: Playwright E2E Tests

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  e2e:
    timeout-minutes: 30
    runs-on: ubuntu-latest

    services:
      mock-api:
        image: proxybot-mock-api
        ports:
          - 8000:8000

      frontend:
        image: proxybot-frontend
        ports:
          - 3000:3000
        env:
          VITE_API_URL: http://localhost:8000

    steps:
      - uses: actions/checkout@v4

      - name: Install dependencies
        run: npm ci

      - name: Install Playwright Browsers
        run: npx playwright install --with-deps chromium

      - name: Run Playwright tests
        run: npm run test:e2e

      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: playwright-report
          path: playwright-report/
          retention-days: 7

      - name: Upload test results
        if: failure()
        run: echo "E2E tests failed. See artifact for details."
"#.to_string()
}

// ============================================================================
// Docker Compose Template (combined)
// ============================================================================

fn generate_docker_compose(_project_name: &str) -> String {
    format!(
        r#"version: '3.8'

services:
  mock-api:
    build:
      context: ./mock-api
      dockerfile: Dockerfile
    ports:
      - "8000:8000"
    environment:
      - PYTHONUNBUFFERED=1
      - DATABASE_URL=postgresql://proxybot:proxybot@postgres:5432/proxybot
    volumes:
      - ./mock-api/fixtures:/app/fixtures:ro
    depends_on:
      postgres:
        condition: service_healthy
    networks:
      - proxybot

  frontend:
    build:
      context: ./frontend
      dockerfile: Dockerfile
    ports:
      - "3000:3000"
    environment:
      - VITE_API_URL=http://mock-api:8000
    depends_on:
      - mock-api
    networks:
      - proxybot

  postgres:
    image: postgres:15-alpine
    ports:
      - "5432:5432"
    environment:
      - POSTGRES_PASSWORD=proxybot
      - POSTGRES_USER=proxybot
      - POSTGRES_DB=proxybot
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql:ro
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U proxybot"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - proxybot

networks:
  proxybot:
    driver: bridge

volumes:
  postgres_data:
"#
    )
}

// ============================================================================
// Frontend Dockerfile
// ============================================================================

fn generate_frontend_dockerfile() -> String {
    r#"FROM node:20-alpine AS builder

WORKDIR /app

COPY package*.json ./
RUN npm ci

COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 3000

CMD ["nginx", "-g", "daemon off;"]
"#.to_string()
}

fn generate_nginx_conf() -> String {
    r#"server {
    listen 3000;
    server_name _;
    root /usr/share/nginx/html;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /api/ {
        proxy_pass http://mock-api:8000/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
"#.to_string()
}

// ============================================================================
// Init SQL
// ============================================================================

fn generate_init_sql() -> String {
    r#"-- ProxyBot deployment initialization
CREATE TABLE IF NOT EXISTS app_stats (
    id SERIAL PRIMARY KEY,
    app_name VARCHAR(100) NOT NULL,
    request_count INTEGER DEFAULT 0,
    bytes_sent BIGINT DEFAULT 0,
    bytes_received BIGINT DEFAULT 0,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_app_stats_name ON app_stats(app_name);
"#.to_string()
}

// ============================================================================
// README
// ============================================================================

fn generate_readme(project_name: &str, mock_endpoints: &[(String, String)], frontend_routes: &[(String, String)]) -> String {
    let mut endpoints_md = String::new();
    for (method, path) in mock_endpoints {
        endpoints_md.push_str(&format!("- **{}** `{}`\n", method, path));
    }

    let mut routes_md = String::new();
    for (component, route) in frontend_routes {
        routes_md.push_str(&format!("- **{}** → `{}`\n", component, route));
    }

    format!(
        r#"# {project_name}

A complete deployment bundle generated by **ProxyBot** — capturing traffic, inferring APIs, and scaffolding a working replica.

## What's Included

### Services

| Service | Port | Description |
|---------|------|-------------|
| `mock-api` | 8000 | FastAPI mock server with recorded responses |
| `frontend` | 3000 | React + Vite frontend scaffold |
| `postgres` | 5432 | PostgreSQL database for persistence |

### Mock API Endpoints

{endpoints}

### Frontend Routes

{routes}

## Quick Start

### Prerequisites

- Docker & Docker Compose
- Node.js 20+ (for local development)

### Run with Docker Compose

```bash
docker compose up --build
```

The frontend will be available at http://localhost:3000

### Run Locally (Development)

**Mock API:**

```bash
cd mock-api
pip install -r requirements.txt
uvicorn main:app --reload --port 8000
```

**Frontend:**

```bash
cd frontend
npm install
npm run dev
```

## Project Structure

```
{project_name}/
├── docker-compose.yml    # Main compose file
├── init.sql             # Database initialization
├── mock-api/            # FastAPI mock server
│   ├── main.py
│   ├── fixtures/
│   ├── Dockerfile
│   └── requirements.txt
├── frontend/            # React scaffold
│   ├── src/
│   ├── Dockerfile
│   └── package.json
└── .github/
    └── workflows/
        └── e2e.yml      # GitHub Actions CI
```

## CI/CD

This project includes a GitHub Actions workflow that runs Playwright E2E tests on every push.

To enable:
1. Push this project to GitHub
2. The workflow will run automatically
3. View results in the "Actions" tab

## Generate a New Bundle

To regenerate this bundle from fresh traffic capture:
1. Capture traffic with ProxyBot
2. Run API inference
3. Generate scaffold
4. Click "Generate Deployment Bundle" in ProxyBot

## License

MIT
"#,
        project_name = project_name,
        endpoints = if endpoints_md.is_empty() { "No endpoints recorded yet.".to_string() } else { endpoints_md },
        routes = if routes_md.is_empty() { "No routes generated yet.".to_string() } else { routes_md }
    )
}

// ============================================================================
// Git Initialization
// ============================================================================

fn init_git_repo(base_path: &PathBuf) -> Result<(), String> {
    // Create .github/workflows directory
    let workflows_dir = base_path.join(".github").join("workflows");
    fs::create_dir_all(&workflows_dir).map_err(|e| format!("Failed to create .github/workflows: {}", e))?;

    // Create .gitignore
    let gitignore = r#"# Dependencies
node_modules/
__pycache__/
*.pyc

# Build artifacts
dist/
build/
*.egg-info/

# Environment
.env
.env.local

# IDE
.idea/
.vscode/
*.swp

# OS
.DS_Store
Thumbs.db

# Test artifacts
playwright-report/
test-results/
*.log

# Docker
.dockerignore
"#;
    fs::write(base_path.join(".gitignore"), gitignore).map_err(|e| format!("Failed to write .gitignore: {}", e))?;

    // Initialize git repo
    let output = std::process::Command::new("git")
        .args(["init"])
        .current_dir(base_path)
        .output()
        .map_err(|e| format!("Failed to run git init: {}", e))?;

    if !output.status.success() {
        return Err(format!("git init failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    // Create initial commit
    let _output = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(base_path)
        .output()
        .map_err(|e| format!("Failed to git add: {}", e))?;

    let _output = std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit: ProxyBot deployment bundle"])
        .current_dir(base_path)
        .output()
        .map_err(|e| format!("Failed to git commit: {}", e))?;

    if !output.status.success() {
        // Non-fatal: just log
        log::warn!("git commit failed (may be empty repo): {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

// ============================================================================
// Helper: Collect inferred APIs
// ============================================================================

fn get_inferred_apis(conn: &rusqlite::Connection, session_id: &str) -> Result<Vec<InferredApi>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, name, method, path, params, auth_required, request_ids, score, created_at \
             FROM inferred_apis WHERE session_id = ?1 ORDER BY id",
        )
        .map_err(|e| e.to_string())?;

    let apis: Vec<InferredApi> = stmt
        .query_map(params![session_id], |row| {
            Ok(InferredApi {
                id: row.get(0)?,
                session_id: row.get(1)?,
                name: row.get(2)?,
                method: row.get(3)?,
                path: row.get(4)?,
                params: row.get(5)?,
                auth_required: row.get::<_, i32>(6)? != 0,
                request_ids: row.get(7)?,
                score: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(apis)
}

// ============================================================================
// Mock API File Generation (subset of mockgen)
// ============================================================================

fn get_mock_endpoints_from_db(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<Vec<(String, String)>, String> {
    let mut stmt = conn
        .prepare("SELECT method, path FROM http_requests WHERE session_id = ?1 GROUP BY method, path ORDER BY path")
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, String)> = stmt
        .query_map(params![session_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rows)
}

// ============================================================================
// Frontend Route Extraction
// ============================================================================

fn get_frontend_routes(conn: &rusqlite::Connection, session_id: &str) -> Result<Vec<(String, String)>, String> {
    let apis = get_inferred_apis(conn, session_id)?;
    let mut routes: Vec<(String, String)> = apis
        .iter()
        .map(|api| {
            let component = format!("{}Page", api.name.replace("/", "_").replace("-", "_"));
            let route = api.path.clone();
            (component, route)
        })
        .collect();
    routes.sort();
    routes.dedup();
    Ok(routes)
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Generate a deployment bundle (in-memory, no files written).
#[tauri::command]
pub fn generate_deployment_bundle(
    db: State<'_, Arc<DbState>>,
    session_id: String,
    project_name: Option<String>,
) -> Result<DeploymentBundle, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let name = project_name.unwrap_or_else(|| "proxybot_deployment".to_string());

    // Get mock endpoints
    let mock_endpoints = get_mock_endpoints_from_db(&conn, &session_id)?;

    // Get frontend routes
    let frontend_routes = get_frontend_routes(&conn, &session_id)?;

    // Generate docker-compose
    let docker_compose_content = generate_docker_compose(&name);

    // Generate README
    let readme_content = generate_readme(
        &name,
        &mock_endpoints,
        &frontend_routes,
    );

    // Generate CI template
    let ci_template_content = generate_github_actions_ci();

    Ok(DeploymentBundle {
        name,
        base_path: String::new(),
        mock_api_path: "./mock-api".to_string(),
        frontend_path: "./frontend".to_string(),
        docker_compose_content,
        readme_content,
        ci_template_content,
    })
}

/// Write deployment bundle to disk, including mock API, frontend scaffold, git init, and CI.
#[tauri::command]
pub fn write_deployment_bundle(
    db: State<'_, Arc<DbState>>,
    session_id: String,
    project_name: Option<String>,
    output_dir: Option<String>,
) -> Result<DeploymentResult, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let name = project_name.unwrap_or_else(|| "proxybot_deployment".to_string());
    let base = output_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.proxybot/deployments/{}", home, name)
    });

    let base_path = PathBuf::from(&base);
    fs::create_dir_all(&base_path).map_err(|e| format!("Failed to create base dir: {}", e))?;

    // Get mock endpoints and frontend routes
    let mock_endpoints = get_mock_endpoints_from_db(&conn, &session_id)?;
    let frontend_routes = get_frontend_routes(&conn, &session_id)?;

    // Write docker-compose.yml
    fs::write(
        base_path.join("docker-compose.yml"),
        generate_docker_compose(&name),
    ).map_err(|e| format!("Failed to write docker-compose.yml: {}", e))?;

    // Write init.sql
    fs::write(
        base_path.join("init.sql"),
        generate_init_sql(),
    ).map_err(|e| format!("Failed to write init.sql: {}", e))?;

    // Write README.md
    fs::write(
        base_path.join("README.md"),
        generate_readme(&name, &mock_endpoints, &frontend_routes),
    ).map_err(|e| format!("Failed to write README.md: {}", e))?;

    // Create .github/workflows and write CI
    let workflows_dir = base_path.join(".github").join("workflows");
    fs::create_dir_all(&workflows_dir).map_err(|e| format!("Failed to create workflows dir: {}", e))?;
    fs::write(
        workflows_dir.join("e2e.yml"),
        generate_github_actions_ci(),
    ).map_err(|e| format!("Failed to write e2e.yml: {}", e))?;

    // Create mock-api directory
    let mock_api_dir = base_path.join("mock-api");
    let fixtures_dir = mock_api_dir.join("fixtures");
    fs::create_dir_all(&fixtures_dir).map_err(|e| format!("Failed to create fixtures dir: {}", e))?;

    // Write mock-api files (simplified FastAPI stub)
    let mock_main = generate_mock_main(&mock_endpoints);
    fs::write(mock_api_dir.join("main.py"), mock_main)
        .map_err(|e| format!("Failed to write main.py: {}", e))?;

    fs::write(
        mock_api_dir.join("requirements.txt"),
        "fastapi>=0.104.0\nuvicorn>=0.24.0\npsycopg2-binary>=2.9.9\n",
    ).map_err(|e| format!("Failed to write requirements.txt: {}", e))?;

    fs::write(
        mock_api_dir.join("Dockerfile"),
        r#"FROM python:3.11-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

EXPOSE 8000

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
"#,
    ).map_err(|e| format!("Failed to write Dockerfile: {}", e))?;

    // Write a placeholder fixture
    let placeholder_fixture = serde_json::json!([
        {"variant_id": "variant_0", "status": 200, "headers": {"Content-Type": "application/json"}, "body": "{\"message\": \"Mock response\"}", "body_type": "json", "order_index": 0}
    ]);
    fs::write(
        fixtures_dir.join("placeholder.json"),
        serde_json::to_string_pretty(&placeholder_fixture).unwrap(),
    ).map_err(|e| format!("Failed to write placeholder fixture: {}", e))?;

    // Create frontend directory
    let frontend_dir = base_path.join("frontend");
    let frontend_src = frontend_dir.join("src");
    fs::create_dir_all(&frontend_src).map_err(|e| format!("Failed to create frontend src: {}", e))?;

    // Write frontend files (simplified React scaffold)
    fs::write(
        frontend_dir.join("package.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "name": "proxybot-frontend",
            "private": true,
            "version": "0.0.1",
            "type": "module",
            "scripts": {
                "dev": "vite",
                "build": "tsc && vite build",
                "preview": "vite preview",
                "test:e2e": "playwright test"
            },
            "dependencies": {
                "react": "^18.2.0",
                "react-dom": "^18.2.0",
                "react-router-dom": "^6.20.0"
            },
            "devDependencies": {
                "@playwright/test": "^1.40.0",
                "@types/react": "^18.2.43",
                "@types/react-dom": "^18.2.17",
                "@vitejs/plugin-react": "^4.2.1",
                "typescript": "^5.3.2",
                "vite": "^5.0.8"
            }
        })).unwrap(),
    ).map_err(|e| format!("Failed to write package.json: {}", e))?;

    fs::write(
        frontend_dir.join("Dockerfile"),
        generate_frontend_dockerfile(),
    ).map_err(|e| format!("Failed to write frontend Dockerfile: {}", e))?;

    fs::write(
        frontend_dir.join("nginx.conf"),
        generate_nginx_conf(),
    ).map_err(|e| format!("Failed to write nginx.conf: {}", e))?;

    fs::write(
        frontend_dir.join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>ProxyBot Frontend</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#,
    ).map_err(|e| format!("Failed to write index.html: {}", e))?;

    fs::write(
        frontend_src.join("main.tsx"),
        r#"import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
"#,
    ).map_err(|e| format!("Failed to write main.tsx: {}", e))?;

    // Generate App.tsx with routes from frontend_routes
    let app_content = generate_frontend_app(&frontend_routes);
    fs::write(frontend_src.join("App.tsx"), app_content)
        .map_err(|e| format!("Failed to write App.tsx: {}", e))?;

    fs::write(
        frontend_src.join("App.css"),
        r#"* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }
.container { max-width: 1200px; margin: 0 auto; padding: 2rem; }
.header { text-align: center; margin-bottom: 2rem; }
.page { background: white; border-radius: 8px; padding: 1.5rem; margin-bottom: 1rem; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }
.page h1 { margin-bottom: 1rem; color: #111; }
.loading, .error { text-align: center; padding: 2rem; }
.error { color: #dc2626; }
"#,
    ).map_err(|e| format!("Failed to write App.css: {}", e))?;

    fs::write(
        frontend_dir.join("tsconfig.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "compilerOptions": {
                "target": "ES2020", "useDefineForClassFields": true,
                "lib": ["ES2020", "DOM", "DOM.Iterable"],
                "module": "ESNext", "skipLibCheck": true,
                "moduleResolution": "bundler", "allowImportingTsExtensions": true,
                "resolveJsonModule": true, "isolatedModules": true,
                "noEmit": true, "jsx": "react-jsx",
                "strict": true, "noUnusedLocals": true,
                "noUnusedParameters": true, "noFallthroughCasesInSwitch": true
            },
            "include": ["src"],
            "references": [{ "path": "./tsconfig.node.json" }]
        })).unwrap(),
    ).map_err(|e| format!("Failed to write tsconfig.json: {}", e))?;

    fs::write(
        frontend_dir.join("tsconfig.node.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "compilerOptions": {
                "composite": true, "skipLibCheck": true,
                "module": "ESNext", "moduleResolution": "bundler",
                "allowSyntheticDefaultImports": true
            },
            "include": ["vite.config.ts"]
        })).unwrap(),
    ).map_err(|e| format!("Failed to write tsconfig.node.json: {}", e))?;

    fs::write(
        frontend_dir.join("vite.config.ts"),
        r#"import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8000',
        changeOrigin: true
      }
    }
  }
})
"#,
    ).map_err(|e| format!("Failed to write vite.config.ts: {}", e))?;

    fs::write(
        frontend_dir.join("playwright.config.ts"),
        serde_json::to_string_pretty(&serde_json::json!({
            "testDir": "./e2e",
            "timeout": 30000,
            "use": {
                "baseURL": "http://localhost:3000",
                "trace": "on-first-retry"
            },
            "projects": [
                { "name": "chromium", "use": { "browserName": "chromium" } }
            ]
        })).unwrap(),
    ).map_err(|e| format!("Failed to write playwright.config.ts: {}", e))?;

    // Write a basic e2e test
    let e2e_dir = frontend_dir.join("e2e");
    fs::create_dir_all(&e2e_dir).map_err(|e| format!("Failed to create e2e dir: {}", e))?;
    fs::write(
        e2e_dir.join("home.spec.ts"),
        r#"import { test, expect } from '@playwright/test'

test('homepage loads', async ({ page }) => {
  await page.goto('/')
  await page.waitForLoadState('networkidle')
  await expect(page.locator('.header')).toBeVisible()
})

test('navigation works', async ({ page }) => {
  await page.goto('/')
  await page.waitForLoadState('networkidle')
  // Basic smoke test
  await expect(page.locator('.container')).toBeVisible()
})
"#,
    ).map_err(|e| format!("Failed to write e2e test: {}", e))?;

    // Initialize git repo
    if let Err(e) = init_git_repo(&base_path) {
        log::warn!("Git init failed (non-fatal): {}", e);
    }

    log::info!("Deployment bundle written to {}", base);

    Ok(DeploymentResult {
        success: true,
        bundle_path: base.clone(),
        message: format!(
            "Deployment bundle created at {}\n\nTo run:\n  cd {}\n  docker compose up --build",
            base, base
        ),
    })
}

/// Generate the FastAPI main.py content for mock API.
fn generate_mock_main(endpoints: &[(String, String)]) -> String {
    let mut code = String::from(
        r#"from fastapi import FastAPI, Request, Response
from fastapi.responses import JSONResponse
import json
from pathlib import Path

app = FastAPI(title="ProxyBot Mock API")

FIXTURES_DIR = Path(__file__).parent / "fixtures"
"#,
    );

    for (method, path) in endpoints {
        let method_lower = method.to_lowercase();
        let path_slug = path.trim_start_matches('/').replace('/', "_").replace('-', "_");
        let fixture_file = format!("fixture_{}.json", path_slug);

        code.push_str(&format!(
            r#"

@app.{method_lower}("{path}")
async def endpoint_{path_slug}(request: Request) -> Response:
    """Mock endpoint for {method} {path}"""
    fixtures_file = FIXTURES_DIR / "{fixture_file}"
    if fixtures_file.exists():
        with open(fixtures_file) as f:
            fixtures = json.load(f)
        variant = fixtures[0] if isinstance(fixtures, list) else fixtures.get("variants", [{{}}])[0]
        body = variant.get("body", "{{}}")
        status = variant.get("status", 200)
        headers = variant.get("headers", {{"Content-Type": "application/json"}})
        return JSONResponse(content=json.loads(body), status_code=status, headers=headers)
    return JSONResponse({{"error": "No fixture found"}}, status_code=404)
"#,
            method_lower = method_lower,
            path = path,
            path_slug = path_slug,
            fixture_file = fixture_file,
            method = method
        ));
    }

    // Add health check
    code.push_str(
        r#"

@app.get("/health")
async def health():
    return {"status": "ok"}
"#,
    );

    code
}

/// Generate the frontend App.tsx with routes.
fn generate_frontend_app(routes: &[(String, String)]) -> String {
    let mut imports = String::new();
    let mut route_elements = String::new();

    for (component, route) in routes {
        let component_lower = component.replace("Page", "").to_lowercase();
        imports.push_str(&format!("import {} from './pages/{}';\n", component, component_lower));
        route_elements.push_str(&format!("      <Route path=\"{}\" element={{<{} />}} />\n", route, component));
    }

    // Add placeholder route if no routes
    if routes.is_empty() {
        imports.push_str("import Home from './pages/home';\n");
        route_elements.push_str("      <Route path=\"/\" element={<Home />} />\n");

        // Create a placeholder home page
        let pages_dir = PathBuf::from("src/pages");
        let _ = fs::create_dir_all(&pages_dir);
        let home_content = r#"export default function Home() {
  return (
    <div className="page">
      <h1>ProxyBot Frontend</h1>
      <p>Welcome! This scaffold was generated from captured traffic.</p>
      <p>Run API inference and scaffold generation to populate the routes.</p>
    </div>
  )
}
"#;
        let _ = fs::write(pages_dir.join("home.tsx"), home_content);
    }

    format!(
        r#"import {{BrowserRouter, Routes, Route}} from 'react-router-dom'
import './App.css'
{imports}

function App() {{
  return (
    <BrowserRouter>
      <div className="container">
        <header className="header">
          <h1>ProxyBot Frontend</h1>
        </header>
        <Routes>
{route_elements}        </Routes>
      </div>
    </BrowserRouter>
  )
}}

export default App
"#,
    )
}
