//! Mock API code generator module for ProxyBot.
//!
//! Generates a working mock API server from OpenAPI spec and recorded responses.
//! Supports ordered sequences, conditional responses, and Docker deployment.

use crate::db::DbState;
use crate::infer::{generate_openapi_spec, InferredApi};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

// ============================================================================
// Types
// ============================================================================

/// Fixture for a single response variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFixture {
    pub variant_id: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub body_type: String,
    pub order_index: usize,
}

/// Conditional response rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalResponse {
    pub condition_field: String,
    pub condition_value: String,
    pub response_variant_id: String,
}

/// Endpoint with all its fixtures and conditional rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEndpoint {
    pub method: String,
    pub path: String,
    pub name: String,
    pub fixtures: Vec<ResponseFixture>,
    pub conditionals: Vec<ConditionalResponse>,
}

/// Generated mock project structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockProject {
    pub name: String,
    pub base_path: String,
    pub endpoints: Vec<MockEndpoint>,
    pub openapi_spec: String,
}

// ============================================================================
// Fixture Extraction
// ============================================================================

/// Get recorded responses grouped by endpoint (method + path).
fn get_endpoint_fixtures(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<HashMap<String, Vec<ResponseFixture>>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT method, path, resp_status, resp_headers, resp_body
             FROM http_requests
             WHERE session_id = ?1
             ORDER BY id ASC",
        )
        .map_err(|e| e.to_string())?;

    let mut fixtures_map: HashMap<String, Vec<ResponseFixture>> = HashMap::new();
    let mut order_index: HashMap<String, usize> = HashMap::new();

    let rows = stmt
        .query_map(params![session_id], |row| {
            let method: String = row.get(0)?;
            let path: String = row.get(1)?;
            let status: Option<u16> = row.get(2)?;
            let headers_json: String = row.get(3)?;
            let body: Option<Vec<u8>> = row.get(4)?;
            Ok((method, path, status, headers_json, body))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (method, path, status, headers_json, body) = row.map_err(|e| e.to_string())?;
        let key = format!("{}:{}", method.to_uppercase(), path);
        let idx = order_index.entry(key.clone()).or_insert(0);
        let current_idx = *idx;
        *idx += 1;

        let headers: HashMap<String, String> =
            serde_json::from_str(&headers_json).unwrap_or_default();
        let body_str = body.as_ref().map(|b| String::from_utf8_lossy(b).to_string());
        let body_type = body_str
            .as_ref()
            .and_then(|s| {
                let trimmed = s.trim();
                if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    Some("json".to_string())
                } else if trimmed.starts_with('<') {
                    Some("xml".to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "text".to_string());

        fixtures_map
            .entry(key)
            .or_insert_with(Vec::new)
            .push(ResponseFixture {
                variant_id: format!("variant_{}", current_idx),
                status: status.unwrap_or(200),
                headers,
                body: body_str,
                body_type,
                order_index: current_idx,
            });
    }

    Ok(fixtures_map)
}

/// Extract conditional rules from request bodies.
/// If multiple requests to same endpoint have different bodies,
/// we treat them as conditional variants.
fn extract_conditionals(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<HashMap<String, Vec<ConditionalResponse>>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT method, path, req_headers, req_body
             FROM http_requests
             WHERE session_id = ?1 AND req_body IS NOT NULL
             ORDER BY id ASC",
        )
        .map_err(|e| e.to_string())?;

    let mut conditionals_map: HashMap<String, Vec<ConditionalResponse>> = HashMap::new();
    let mut variant_index: HashMap<String, usize> = HashMap::new();

    let rows = stmt
        .query_map(params![session_id], |row| {
            let method: String = row.get(0)?;
            let path: String = row.get(1)?;
            let headers_json: String = row.get(2)?;
            let body: Option<Vec<u8>> = row.get(3)?;
            Ok((method, path, headers_json, body))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (method, path, _headers_json, body) = row.map_err(|e| e.to_string())?;
        let key = format!("{}:{}", method.to_uppercase(), path);

        let body_str = match body {
            Some(b) => String::from_utf8_lossy(&b).to_string(),
            None => continue,
        };

        // Try to parse as JSON to find condition fields
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body_str) {
            if let Some(obj) = parsed.as_object() {
                // Use first string field as condition field
                for (field, value) in obj {
                    if let Some(val_str) = value.as_str() {
                        let variant_idx = variant_index.entry(key.clone()).or_insert(0);
                        let current_idx = *variant_idx;
                        *variant_idx += 1;

                        let cond = ConditionalResponse {
                            condition_field: field.clone(),
                            condition_value: val_str.to_string(),
                            response_variant_id: format!("variant_{}", current_idx),
                        };

                        conditionals_map
                            .entry(key.clone())
                            .or_insert_with(Vec::new)
                            .push(cond);
                        break;
                    }
                }
            }
        }
    }

    Ok(conditionals_map)
}

// ============================================================================
// Code Generation
// ============================================================================

/// Generate FastAPI main.py content.
fn generate_fastapi_main(endpoints: &[MockEndpoint]) -> String {
    let mut code = String::from(
        "from fastapi import FastAPI, Request, Response\n\
         from fastapi.responses import JSONResponse, PlainTextResponse\n\
         import json\n\
         import os\n\
         from pathlib import Path\n\
         \n\
         app = FastAPI(title=\"ProxyBot Mock API\")\n\
         \n\
         # Load fixtures\n\
         FIXTURES_DIR = Path(__file__).parent / \"fixtures\"\n\
         \n",
    );

    for endpoint in endpoints {
        let method_lower = endpoint.method.to_lowercase();
        let path_slug = endpoint
            .path
            .trim_start_matches('/')
            .replace('/', "_")
            .replace('-', "_");

        // Build fixture file path (without format! to avoid {} issues)
        let fixture_file_name = format!("fixture_{}.json", path_slug);

        // Add endpoint function
        code.push_str("\n\n");
        code.push_str(&format!(
            r#"@app.{}( "{}" )
async def endpoint_{}(request: Request) -> Response:
    """{} - {} {}"""
    "#,
            method_lower,
            endpoint.path,
            path_slug,
            endpoint.name,
            endpoint.method,
            endpoint.path
        ));

        // Build response logic based on conditional or ordered
        let has_conditionals = !endpoint.conditionals.is_empty();
        if !has_conditionals {
            // Ordered sequence: cycle through variants
            code.push_str(&format!(
                r#"    fixtures_file = FIXTURES_DIR / "{}"
    if fixtures_file.exists():

        with open(fixtures_file) as f:
            fixtures = json.load(f)

        # Ordered sequence: track call count for cycling
        call_key = "{}"
        if not hasattr(app.state, 'call_counts'):
            app.state.call_counts = {{}}
        if call_key not in app.state.call_counts:
            app.state.call_counts[call_key] = 0
        idx = app.state.call_counts[call_key] % len(fixtures)
        app.state.call_counts[call_key] += 1

        variant = fixtures[idx]
        headers = variant.get("headers", {{}})
        body = variant.get("body")
        status = variant.get("status", 200)

        if body:
            return JSONResponse(content=json.loads(body), status_code=status, headers=headers)
        else:
            return Response(content="", status_code=status, headers=headers)

    return JSONResponse({{"error": "No fixture found"}}, status_code=404)
"#,
                fixture_file_name,
                path_slug
            ));
        } else {
            // Conditional: match request body field → response variant
            code.push_str(&format!(
                r#"    fixtures_file = FIXTURES_DIR / "{}"
    if fixtures_file.exists():

        with open(fixtures_file) as f:
            fixtures = json.load(f)

        # Get request body
        body_str = (await request.body()).decode()
        try:
            body_json = json.loads(body_str)
        except:
            body_json = {{}}

        # Find matching conditional variant
        matched = False
        for cond in fixtures.get("conditionals", []):
            if body_json.get(cond["condition_field"]) == cond["condition_value"]:
                variant_id = cond["response_variant_id"]
                for variant in fixtures.get("variants", []):
                    if variant["variant_id"] == variant_id:
                        headers = variant.get("headers", {{}})
                        body = variant.get("body")
                        status = variant.get("status", 200)
                        if body:
                            return JSONResponse(content=json.loads(body), status_code=status, headers=headers)
                        else:
                            return Response(content="", status_code=status, headers=headers)
                        matched = True
                        break
                if matched:
                    break

        if not matched:
            # No match: return default response or ordered fallback
            if len(fixtures.get("variants", [])) > 0:
                variant = fixtures["variants"][0]
                headers = variant.get("headers", {{}})
                body = variant.get("body")
                status = variant.get("status", 200)
                if body:
                    return JSONResponse(content=json.loads(body), status_code=status, headers=headers)
                else:
                    return Response(content="", status_code=status, headers=headers)
            else:
                return JSONResponse({{"error": "No matching conditional"}}, status_code=404)

    return JSONResponse({{"error": "No fixture found"}}, status_code=404)
"#,
                fixture_file_name
            ));
        }
    }

    code
}

/// Generate fixture JSON for an endpoint.
fn generate_fixture_json(
    endpoint: &MockEndpoint,
    conditionals: &[ConditionalResponse],
) -> serde_json::Value {
    let variants: Vec<serde_json::Value> = endpoint
        .fixtures
        .iter()
        .map(|f| {
            serde_json::json!({
                "variant_id": f.variant_id,
                "status": f.status,
                "headers": f.headers,
                "body": f.body,
                "body_type": f.body_type,
                "order_index": f.order_index,
            })
        })
        .collect();

    if conditionals.is_empty() {
        // Simple array for ordered sequence
        serde_json::json!(variants)
    } else {
        // Object with variants and conditionals for conditional response
        serde_json::json!({
            "variants": variants,
            "conditionals": conditionals,
        })
    }
}

/// Generate Dockerfile content.
fn generate_dockerfile() -> String {
    String::from(
        "FROM python:3.11-slim\n\
         \n\
         WORKDIR /app\n\
         \n\
         COPY requirements.txt .\n\
         RUN pip install --no-cache-dir -r requirements.txt\n\
         \n\
         COPY . .\n\
         \n\
         EXPOSE 8000\n\
         \n\
         CMD [\"uvicorn\", \"main:app\", \"--host\", \"0.0.0.0\", \"--port\", \"8000\"]\n",
    )
}

/// Generate requirements.txt content.
fn generate_requirements() -> String {
    String::from("fastapi>=0.104.0\nuvicorn>=0.24.0\n")
}

/// Generate docker-compose.yml content.
fn generate_docker_compose() -> String {
    String::from(
        "version: '3.8'\n\
         \n\
         services:\n\
         \n\
         mock-api:\n\
         build: .\n\
         ports:\n\
         - \"8000:8000\"\n\
         environment:\n\
         - PYTHONUNBUFFERED=1\n\
         volumes:\n\
         - ./fixtures:/app/fixtures:ro\n\
         \n\
         postgres:\n\
         image: postgres:15-alpine\n\
         ports:\n\
         - \"5432:5432\"\n\
         environment:\n\
         - POSTGRES_PASSWORD=proxybot\n\
         - POSTGRES_USER=proxybot\n\
         - POSTGRES_DB=proxybot\n\
         volumes:\n\
         - postgres_data:/var/lib/postgresql/data\n\
         \n\
         volumes:\n\
         postgres_data:\n",
    )
}

/// Generate README content.
fn generate_readme(project_name: &str, endpoints: &[MockEndpoint]) -> String {
    let mut endpoint_list = String::new();
    for ep in endpoints {
        endpoint_list.push_str(&format!("- **{}** `{} {}`\n", ep.name, ep.method, ep.path));
    }

    format!(
        "# {} Mock API\n\
         \n\
         Generated by ProxyBot Mock API Generator.\n\
         \n\
         ## Endpoints\n\
         \n\
         {}\n\
         \n\
         ## Running\n\
         \n\
         ### With Docker Compose\n\
         \n\
         ```bash\n\
         docker compose up --build\n\
         ```\n\
         \n\
         ### Local Development\n\
         \n\
         ```bash\n\
         pip install -r requirements.txt\n\
         uvicorn main:app --reload\n\
         ```\n\
         \n\
         ## Testing\n\
         \n\
         ```bash\n\
         curl http://localhost:8000/<endpoint-path>\n\
         ```\n\
         \n\
         ## Fixtures\n\
         \n\
         Fixtures are stored in the `fixtures/` directory.\n\
         Each endpoint has its own fixture file with response variants.\n\
         ",
        project_name, endpoint_list
    )
}

// ============================================================================
// Helper to collect inferred APIs
// ============================================================================

fn get_inferred_apis(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<Vec<InferredApi>, String> {
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
// Tauri Commands
// ============================================================================

/// Generate a mock API project from session traffic.
#[tauri::command]
pub fn generate_mock_project(
    db_state: State<'_, Arc<DbState>>,
    session_id: String,
    project_name: Option<String>,
) -> Result<MockProject, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    // Get inferred APIs for this session
    let apis = get_inferred_apis(&conn, &session_id)?;

    if apis.is_empty() {
        return Err("No inferred APIs found for this session. Run API inference first.".to_string());
    }

    // Get fixtures and conditionals
    let fixtures_map = get_endpoint_fixtures(&conn, &session_id)?;
    let conditionals_map = extract_conditionals(&conn, &session_id)?;

    // Build mock endpoints
    let mut endpoints = Vec::new();
    for api in &apis {
        let key = format!("{}:{}", api.method.to_uppercase(), api.path);
        let fixtures = fixtures_map.get(&key).cloned().unwrap_or_default();
        let conditionals = conditionals_map.get(&key).cloned().unwrap_or_default();

        endpoints.push(MockEndpoint {
            method: api.method.clone(),
            path: api.path.clone(),
            name: api.name.clone(),
            fixtures,
            conditionals,
        });
    }

    // Generate OpenAPI spec
    let spec = generate_openapi_spec(&apis, &[], &format!("{} Mock API", project_name.as_deref().unwrap_or("ProxyBot")));
    let spec_str = serde_json::to_string_pretty(&spec).map_err(|e| e.to_string())?;

    Ok(MockProject {
        name: project_name.unwrap_or_else(|| "proxybot_mock".to_string()),
        base_path: String::new(),
        endpoints,
        openapi_spec: spec_str,
    })
}

/// Write mock project to disk.
#[tauri::command]
pub fn write_mock_project(
    db_state: State<'_, Arc<DbState>>,
    session_id: String,
    project_name: Option<String>,
    output_dir: Option<String>,
) -> Result<String, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    // Get inferred APIs
    let apis = get_inferred_apis(&conn, &session_id)?;

    if apis.is_empty() {
        return Err("No inferred APIs found for this session.".to_string());
    }

    let name = project_name.unwrap_or_else(|| "proxybot_mock".to_string());
    let base = output_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.proxybot/mock_projects/{}", home, name)
    });

    let base_path = PathBuf::from(&base);
    let fixtures_dir = base_path.join("fixtures");
    fs::create_dir_all(&fixtures_dir).map_err(|e| e.to_string())?;

    // Get fixtures and conditionals
    let fixtures_map = get_endpoint_fixtures(&conn, &session_id)?;
    let conditionals_map = extract_conditionals(&conn, &session_id)?;

    // Build endpoints and write files
    let mut endpoints = Vec::new();
    for api in &apis {
        let key = format!("{}:{}", api.method.to_uppercase(), api.path);
        let fixtures = fixtures_map.get(&key).cloned().unwrap_or_default();
        let conditionals = conditionals_map.get(&key).cloned().unwrap_or_default();

        // Write fixture file
        let path_slug = api
            .path
            .trim_start_matches('/')
            .replace('/', "_")
            .replace('-', "_");
        let fixture_path = fixtures_dir.join(format!("fixture_{}.json", path_slug));
        let fixture_json = generate_fixture_json(&MockEndpoint {
            method: api.method.clone(),
            path: api.path.clone(),
            name: api.name.clone(),
            fixtures: fixtures.clone(),
            conditionals: conditionals.clone(),
        }, &conditionals);
        fs::write(&fixture_path, serde_json::to_string_pretty(&fixture_json).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

        endpoints.push(MockEndpoint {
            method: api.method.clone(),
            path: api.path.clone(),
            name: api.name.clone(),
            fixtures,
            conditionals,
        });
    }

    // Write main.py
    let main_path = base_path.join("main.py");
    fs::write(&main_path, generate_fastapi_main(&endpoints)).map_err(|e| e.to_string())?;

    // Write supporting files
    fs::write(base_path.join("Dockerfile"), generate_dockerfile()).map_err(|e| e.to_string())?;
    fs::write(base_path.join("requirements.txt"), generate_requirements()).map_err(|e| e.to_string())?;
    fs::write(base_path.join("docker-compose.yml"), generate_docker_compose()).map_err(|e| e.to_string())?;
    fs::write(
        base_path.join("README.md"),
        generate_readme(&name, &endpoints),
    )
    .map_err(|e| e.to_string())?;

    // Write OpenAPI spec
    let spec = generate_openapi_spec(&apis, &[], &format!("{} Mock API", name));
    let spec_str = serde_json::to_string_pretty(&spec).map_err(|e| e.to_string())?;
    fs::write(base_path.join("openapi.json"), &spec_str).map_err(|e| e.to_string())?;

    log::info!("Mock project written to {}", base);

    Ok(base)
}

/// Get mock endpoints info without writing files.
#[tauri::command]
pub fn get_mock_endpoints(
    db_state: State<'_, Arc<DbState>>,
    session_id: String,
) -> Result<Vec<MockEndpoint>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    // Get inferred APIs
    let apis = get_inferred_apis(&conn, &session_id)?;

    if apis.is_empty() {
        return Err("No inferred APIs found for this session.".to_string());
    }

    let fixtures_map = get_endpoint_fixtures(&conn, &session_id)?;
    let conditionals_map = extract_conditionals(&conn, &session_id)?;

    let mut endpoints = Vec::new();
    for api in apis {
        let key = format!("{}:{}", api.method.to_uppercase(), api.path);
        let fixtures = fixtures_map.get(&key).cloned().unwrap_or_default();
        let conditionals = conditionals_map.get(&key).cloned().unwrap_or_default();

        endpoints.push(MockEndpoint {
            method: api.method,
            path: api.path,
            name: api.name,
            fixtures,
            conditionals,
        });
    }

    Ok(endpoints)
}

/// Start the mock server from a generated project for testing.
#[tauri::command]
pub async fn start_mock_server(project_path: String, port: Option<u16>) -> Result<String, String> {
    let port = port.unwrap_or(8000);
    let main_path = PathBuf::from(&project_path).join("main.py");

    if !main_path.exists() {
        return Err(format!("main.py not found at {}. Run write_mock_project first.", project_path));
    }

    // Spawn uvicorn in background
    tokio::process::Command::new("uvicorn")
        .args(["main:app", "--host", "0.0.0.0", "--port", &port.to_string()])
        .current_dir(&project_path)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(format!("Mock server started on port {}", port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_fastapi_main() {
        let endpoints = vec![
            MockEndpoint {
                method: "GET".to_string(),
                path: "/api/user".to_string(),
                name: "getUser".to_string(),
                fixtures: vec![
                    ResponseFixture {
                        variant_id: "variant_0".to_string(),
                        status: 200,
                        headers: [("Content-Type".to_string(), "application/json".to_string())].into(),
                        body: Some(r#"{"id": 1, "name": "test"}"#.to_string()),
                        body_type: "json".to_string(),
                        order_index: 0,
                    },
                ],
                conditionals: vec![],
            },
        ];

        let code = generate_fastapi_main(&endpoints);
        assert!(code.contains("endpoint_api_user"));
        assert!(code.contains("@app.get"));
    }

    #[test]
    fn test_generate_fixture_json_ordered() {
        let endpoint = MockEndpoint {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            name: "test".to_string(),
            fixtures: vec![
                ResponseFixture {
                    variant_id: "variant_0".to_string(),
                    status: 200,
                    headers: HashMap::new(),
                    body: Some("{}".to_string()),
                    body_type: "json".to_string(),
                    order_index: 0,
                },
            ],
            conditionals: vec![],
        };

        let json = generate_fixture_json(&endpoint, &[]);
        assert!(json.is_array());
        assert_eq!(json[0]["variant_id"], "variant_0");
    }

    #[test]
    fn test_generate_fixture_json_conditional() {
        let endpoint = MockEndpoint {
            method: "POST".to_string(),
            path: "/api/test".to_string(),
            name: "testConditional".to_string(),
            fixtures: vec![
                ResponseFixture {
                    variant_id: "variant_0".to_string(),
                    status: 200,
                    headers: HashMap::new(),
                    body: Some("{}".to_string()),
                    body_type: "json".to_string(),
                    order_index: 0,
                },
            ],
            conditionals: vec![
                ConditionalResponse {
                    condition_field: "type".to_string(),
                    condition_value: "admin".to_string(),
                    response_variant_id: "variant_0".to_string(),
                },
            ],
        };

        let json = generate_fixture_json(&endpoint, &endpoint.conditionals);
        assert!(json.is_object());
        assert!(json["variants"].is_array());
        assert!(json["conditionals"].is_array());
    }

    #[test]
    fn test_generate_docker_compose() {
        let compose = generate_docker_compose();
        assert!(compose.contains("mock-api"));
        assert!(compose.contains("postgres"));
        assert!(compose.contains("8000:8000"));
    }
}
