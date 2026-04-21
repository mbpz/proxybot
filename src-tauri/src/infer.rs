//! LLM API semantic inference module.
//!
//! Calls Claude API with traffic data to infer API semantics.
//! Output: JSON with interfaces and modules, stored in inferred_apis table.

use crate::db::DbState;
use crate::normalize::{normalize_http_record, NormalizedRecord};
use reqwest::Client;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use tauri::State;

// ============================================================================
// Types
// ============================================================================

/// Inferred API interface from LLM analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInterface {
    pub name: String,
    pub method: String,
    pub path: String,
    pub params: String,
    pub auth_required: bool,
}

/// Inferred module grouping related interfaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiModule {
    pub name: String,
    pub description: String,
    pub interface_ids: Vec<String>,
}

/// LLM inference result with validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub interfaces: Vec<ApiInterface>,
    pub modules: Vec<ApiModule>,
    pub valid: bool,
    pub errors: Vec<String>,
    pub score: f64,
}

/// Evaluation result from validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub score: f64,
}

/// Inferred API stored in database.
#[derive(Debug, Clone, Serialize)]
pub struct InferredApi {
    pub id: i64,
    pub session_id: String,
    pub name: String,
    pub method: String,
    pub path: String,
    pub params: String,
    pub auth_required: bool,
    pub request_ids: String,
    pub score: Option<f64>,
    pub created_at: String,
}

// ============================================================================
// OpenAPI Generation
// ============================================================================

/// Generate OpenAPI 3.1 spec from inferred APIs.
pub fn generate_openapi_spec(
    apis: &[InferredApi],
    modules: &[ApiModule],
    title: &str,
) -> Value {
    let mut paths = serde_json::Map::new();

    for api in apis {
        let method_lower = api.method.to_lowercase();
        let path_key = api.path.clone();

        let operation = json!({
            "summary": api.name,
            "responses": {
                "200": {
                    "description": "Successful response",
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object"
                            }
                        }
                    }
                }
            }
        });

        if let Some(path_obj) = paths.get_mut(&path_key) {
            if let Some(obj) = path_obj.as_object_mut() {
                obj.insert(method_lower, operation);
                continue;
            }
        }
        paths.insert(path_key, json!({ method_lower: operation }));
    }

    let components = if modules.is_empty() {
        json!({})
    } else {
        let schemas = modules.iter().enumerate().map(|(i, _m)| {
            let schema = json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "description": { "type": "string" }
                }
            });
            (format!("Module{}", i + 1), schema)
        }).collect::<serde_json::Map<String, Value>>();

        json!({ "schemas": schemas })
    };

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": title,
            "version": "1.0.0"
        },
        "paths": paths,
        "components": components
    })
}

// ============================================================================
// LLM Prompt Building
// ============================================================================

fn build_inference_prompt(records: &[NormalizedRecord]) -> String {
    let mut prompt = String::from(
        "Given these HTTP requests, output JSON with: \
        interfaces [{name, method, path, params (semantic description), auth_required}] \
        and modules [{name, description, interface_ids}].\n\n",
    );
    prompt.push_str("HTTP Requests:\n");

    for (i, rec) in records.iter().take(100).enumerate() {
        prompt.push_str(&format!(
            "{}. {} {} - Status: {}\n",
            i + 1,
            rec.method,
            rec.path,
            rec.response_status
        ));
        if !rec.request_headers.is_object() || rec.request_body != Value::Null {
            prompt.push_str(&format!(
                "   Headers: {}\n",
                serde_json::to_string(&rec.request_headers).unwrap_or_default()
            ));
        }
        if rec.request_body != Value::Null {
            prompt.push_str(&format!(
                "   Body: {}\n",
                serde_json::to_string(&rec.request_body).unwrap_or_default()
            ));
        }
        if rec.response_body != Value::Null {
            let body_str = serde_json::to_string(&rec.response_body).unwrap_or_default();
            if body_str.len() < 500 {
                prompt.push_str(&format!("   Response: {}\n", body_str));
            }
        }
    }

    prompt.push_str(
        "\nOutput JSON format: \
        {\"interfaces\": [{\"name\": \"...\", \"method\": \"GET|POST|...\", \
        \"path\": \"/api/...\", \"params\": \"...\", \"auth_required\": true|false}], \
        \"modules\": [{\"name\": \"...\", \"description\": \"...\", \
        \"interface_ids\": [\"name1\", \"name2\"]}]}",
    );

    prompt
}

fn build_evaluation_prompt(inference: &InferenceResult) -> String {
    let interfaces_json = serde_json::to_string_pretty(&inference.interfaces).unwrap_or_default();
    let modules_json = serde_json::to_string_pretty(&inference.modules).unwrap_or_default();

    format!(
        "Review the inferred API JSON. Check: all requests are covered, \
        paths are correctly typed, auth chains are consistent, parameter names match request bodies.\n\n\
        Interfaces:\n{}\n\nModules:\n{}\n\n\
        Output JSON: {{\"valid\": true|false, \"errors\": [\"...\"], \"score\": 0.0-1.0}}",
        interfaces_json, modules_json
    )
}

// ============================================================================
// HTTP Client for Claude API
// ============================================================================

fn get_anthropic_api_key() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("CLAUDE_API_KEY"))
        .ok()
}

async fn call_claude_api(prompt: &str, api_key: &str) -> Result<String, String> {
    let client = Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&json!({
            "model": "claude-sonnet-4-7-20251101",
            "max_tokens": 4096,
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        }))
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    #[derive(Deserialize)]
    struct ApiResponse {
        content: Vec<ContentBlock>,
    }
    #[derive(Deserialize)]
    struct ContentBlock {
        #[serde(rename = "type")]
        block_type: String,
        text: Option<String>,
    }

    let api_resp: ApiResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    for block in api_resp.content {
        if block.block_type == "text" {
            if let Some(text) = block.text {
                return Ok(text);
            }
        }
    }

    Err("No text content in API response".to_string())
}

// ============================================================================
// Validation
// ============================================================================

fn validate_inference_result(result: &mut InferenceResult) {
    let mut errors = Vec::new();

    // Check interfaces
    let interface_names: Vec<&str> = result.interfaces.iter().map(|i| i.name.as_str()).collect();
    for interface in &result.interfaces {
        if interface.method.is_empty() {
            errors.push(format!("Interface '{}' has empty method", interface.name));
        }
        if !interface.path.starts_with('/') {
            errors.push(format!("Interface '{}' path must start with /", interface.name));
        }
        if interface.name.is_empty() {
            errors.push("Interface has empty name".to_string());
        }
        // Check for duplicate names
        let count = interface_names.iter().filter(|n| **n == interface.name).count();
        if count > 1 {
            errors.push(format!("Duplicate interface name '{}'", interface.name));
        }
    }

    // Check modules
    for module in &result.modules {
        if module.name.is_empty() {
            errors.push("Module has empty name".to_string());
        }
        for iface_id in &module.interface_ids {
            if !interface_names.iter().any(|n| *n == iface_id.as_str()) {
                errors.push(format!(
                    "Module '{}' references unknown interface '{}'",
                    module.name, iface_id
                ));
            }
        }
    }

    let score = if errors.is_empty() {
        1.0
    } else {
        let penalty = errors.len() as f64 * 0.1;
        (1.0 - penalty).max(0.0)
    };

    result.valid = errors.is_empty();
    result.errors = errors;
    result.score = score;
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Infer API semantics from traffic using LLM.
#[tauri::command]
pub async fn infer_api_semantics(
    db_state: State<'_, Arc<DbState>>,
    session_id: Option<String>,
    device_id: Option<i64>,
) -> Result<InferenceResult, String> {
    let api_key = get_anthropic_api_key()
        .ok_or("ANTHROPIC_API_KEY not set")?;

    // Get traffic records from database
    let records = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

        let mut conditions = Vec::new();
        if let Some(ref sid) = session_id {
            conditions.push(format!("session_id = '{}'", sid));
        }
        if let Some(d) = device_id {
            conditions.push(format!("device_id = {}", d));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let query = format!(
            "SELECT id, timestamp, method, path, req_headers, req_body, resp_status, \
             resp_headers, resp_body, duration_ms, device_id \
             FROM http_requests{} ORDER BY id DESC LIMIT 100",
            where_clause
        );

        let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
        let records = stmt
            .query_map(params![], |row| {
                let id: i64 = row.get(0)?;
                let timestamp: String = row.get(1)?;
                let method: String = row.get(2)?;
                let path: String = row.get(3)?;
                let req_headers: String = row.get(4)?;
                let req_body: Option<Vec<u8>> = row.get(5)?;
                let resp_status: Option<i64> = row.get(6)?;
                let resp_headers: String = row.get(7)?;
                let resp_body: Option<Vec<u8>> = row.get(8)?;
                let duration_ms: Option<i64> = row.get(9)?;
                let device_id: Option<i64> = row.get(10)?;

                Ok(normalize_http_record(
                    id,
                    &timestamp,
                    &method,
                    &path,
                    &req_headers,
                    req_body.as_deref(),
                    resp_status,
                    &resp_headers,
                    resp_body.as_deref(),
                    duration_ms,
                    device_id,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        records
    };

    if records.is_empty() {
        return Err("No traffic records to analyze".to_string());
    }

    // Build prompt and call LLM
    let prompt = build_inference_prompt(&records);
    let llm_output = call_claude_api(&prompt, &api_key).await?;

    // Parse LLM output as JSON
    let mut result: InferenceResult = serde_json::from_str(&llm_output)
        .map_err(|e| format!("Failed to parse LLM output as JSON: {}. Output was: {}", e, &llm_output))?;

    // Validate result
    validate_inference_result(&mut result);

    // Retry up to 2 times if score < 0.8
    let mut retries = 0;
    while result.score < 0.8 && retries < 2 {
        let eval_prompt = build_evaluation_prompt(&result);
        let eval_output = call_claude_api(&eval_prompt, &api_key).await?;

        if let Ok(eval_result) = serde_json::from_str::<EvaluationResult>(&eval_output) {
            result.score = eval_result.score;
            result.errors = eval_result.errors.clone();
            result.valid = eval_result.valid;
        }

        retries += 1;
    }

    Ok(result)
}

/// Store inference result in database.
#[tauri::command]
pub fn store_inference_result(
    db_state: State<'_, Arc<DbState>>,
    session_id: String,
    inference: InferenceResult,
) -> Result<Vec<i64>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono_lite_timestamp();
    let mut ids = Vec::new();

    for interface in &inference.interfaces {
        conn.execute(
            "INSERT INTO inferred_apis \
             (session_id, name, method, path, params, auth_required, request_ids, score, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                session_id,
                interface.name,
                interface.method,
                interface.path,
                interface.params,
                interface.auth_required as i32,
                "[]",
                inference.score,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        ids.push(id);
    }

    Ok(ids)
}

/// Get stored inference results.
#[tauri::command]
pub fn get_inferred_apis(
    db_state: State<'_, Arc<DbState>>,
    session_id: Option<String>,
) -> Result<Vec<InferredApi>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    let query = "SELECT id, session_id, name, method, path, params, auth_required, request_ids, score, created_at \
                 FROM inferred_apis WHERE (?1 = '' OR session_id = ?1) ORDER BY id";

    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;

    let sid_param = session_id.unwrap_or_default();
    let apis = stmt
        .query_map(params![sid_param], |row| {
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

/// Get OpenAPI spec from inferred APIs.
#[tauri::command]
pub fn get_openapi_spec(
    db_state: State<'_, Arc<DbState>>,
    session_id: String,
) -> Result<String, String> {
    let apis = get_inferred_apis(db_state, Some(session_id.clone()))?;
    let spec = generate_openapi_spec(&apis, &[], "ProxyBot Traffic API");
    serde_json::to_string_pretty(&spec).map_err(|e| e.to_string())
}

/// Generate OpenAPI YAML spec.
#[tauri::command]
pub fn generate_openapi_yaml(
    db_state: State<'_, Arc<DbState>>,
    session_id: String,
) -> Result<String, String> {
    let apis = get_inferred_apis(db_state, Some(session_id.clone()))?;
    let spec = generate_openapi_spec(&apis, &[], "ProxyBot Traffic API");
    serde_yaml::to_string(&spec).map_err(|e| e.to_string())
}

/// Evaluate stored inference results using LLM.
#[tauri::command]
pub async fn evaluate_inference(
    db_state: State<'_, Arc<DbState>>,
    session_id: String,
) -> Result<EvaluationResult, String> {
    let api_key = get_anthropic_api_key()
        .ok_or("ANTHROPIC_API_KEY not set")?;

    // Get stored inferences for the session directly
    let apis = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        let query = "SELECT id, session_id, name, method, path, params, auth_required, request_ids, score, created_at \
                     FROM inferred_apis WHERE session_id = ?1 ORDER BY id";

        let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;

        let result: Result<Vec<InferredApi>, _> = stmt.query_map(params![session_id], |row| {
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
        .collect();

        result.map_err(|e| e.to_string())?
    };

    if apis.is_empty() {
        return Err("No inferred APIs found for this session".to_string());
    }

    // Build interfaces list for evaluation
    let interfaces: Vec<ApiInterface> = apis.iter().map(|a| ApiInterface {
        name: a.name.clone(),
        method: a.method.clone(),
        path: a.path.clone(),
        params: a.params.clone(),
        auth_required: a.auth_required,
    }).collect();

    // Build inference result for evaluation
    let inference = InferenceResult {
        interfaces,
        modules: Vec::new(), // Modules not stored yet
        valid: false,
        errors: Vec::new(),
        score: 0.0,
    };

    // Build evaluation prompt and call LLM
    let eval_prompt = build_evaluation_prompt(&inference);
    let eval_output = call_claude_api(&eval_prompt, &api_key).await?;

    // Parse evaluation result
    let mut eval_result: EvaluationResult = serde_json::from_str(&eval_output)
        .map_err(|e| format!("Failed to parse evaluation result as JSON: {}. Output was: {}", e, &eval_output))?;

    // Store evaluation result in database
    {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        let now = chrono_lite_timestamp();
        let errors_json = serde_json::to_string(&eval_result.errors).unwrap_or_default();

        conn.execute(
            "INSERT INTO inference_evaluations (session_id, valid, errors, score, evaluated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session_id,
                eval_result.valid as i32,
                errors_json,
                eval_result.score,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    // Retry up to 2 times if score < 0.8
    let mut retries = 0;
    while eval_result.score < 0.8 && retries < 2 {
        // Build feedback prompt with current errors
        let feedback_prompt = format!(
            "{}\n\nPrevious evaluation score was {:.1}. Errors found: {:?}. \
             Please re-evaluate with these corrections in mind.",
            eval_prompt,
            eval_result.score,
            eval_result.errors
        );

        let retry_output = call_claude_api(&feedback_prompt, &api_key).await?;

        if let Ok(new_result) = serde_json::from_str::<EvaluationResult>(&retry_output) {
            eval_result = new_result;

            // Update stored evaluation
            let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
            let errors_json = serde_json::to_string(&eval_result.errors).unwrap_or_default();

            conn.execute(
                "INSERT INTO inference_evaluations (session_id, valid, errors, score, evaluated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    session_id,
                    eval_result.valid as i32,
                    errors_json,
                    eval_result.score,
                    chrono_lite_timestamp(),
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        retries += 1;
    }

    Ok(eval_result)
}

/// Get evaluation result for a session.
#[tauri::command]
pub fn get_evaluation_result(
    db_state: State<'_, Arc<DbState>>,
    session_id: String,
) -> Result<Option<EvaluationResult>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    let query = "SELECT valid, errors, score, evaluated_at \
                 FROM inference_evaluations WHERE session_id = ?1 \
                 ORDER BY evaluated_at DESC LIMIT 1";

    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;

    let result = stmt.query_row(params![session_id], |row| {
        let valid: i32 = row.get(0)?;
        let errors_json: String = row.get(1)?;
        let score: f64 = row.get(2)?;
        let _evaluated_at: String = row.get(3)?;

        let errors: Vec<String> = serde_json::from_str(&errors_json).unwrap_or_default();

        Ok(EvaluationResult {
            valid: valid != 0,
            errors,
            score,
        })
    });

    match result {
        Ok(eval) => Ok(Some(eval)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

// ============================================================================
// Utility
// ============================================================================

fn chrono_lite_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    let mut remaining = secs;

    let mut year = 1970;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year * 86400 {
            break;
        }
        remaining -= days_in_year * 86400;
        year += 1;
    }

    let days_in_months: &[u64] = if is_leap_year(year) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    for days in days_in_months {
        if remaining < days * 86400 {
            break;
        }
        remaining -= days * 86400;
        month += 1;
    }

    let day = (remaining / 86400) + 1;
    remaining %= 86400;
    let hour = remaining / 3600;
    remaining %= 3600;
    let minute = remaining / 60;
    let second = remaining % 60;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    )
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let apis = vec![InferredApi {
            id: 1,
            session_id: "test".to_string(),
            name: "getUser".to_string(),
            method: "GET".to_string(),
            path: "/api/user".to_string(),
            params: "user ID".to_string(),
            auth_required: true,
            request_ids: "[]".to_string(),
            score: Some(0.9),
            created_at: "2024-01-01".to_string(),
        }];
        let spec = generate_openapi_spec(&apis, &[], "Test API");
        assert_eq!(spec["openapi"], "3.1.0");
        assert!(spec["paths"].as_object().unwrap().contains_key("/api/user"));
    }

    #[test]
    fn test_build_inference_prompt() {
        let records = vec![NormalizedRecord {
            id: 1,
            timestamp: "2024-01-01".to_string(),
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            query: json!({}),
            request_headers: json!({}),
            request_body: Value::Null,
            response_status: 200,
            response_headers: json!({}),
            response_body: Value::Null,
            timing_ms: 100,
            device_id: None,
        }];
        let prompt = build_inference_prompt(&records);
        assert!(prompt.contains("GET"));
        assert!(prompt.contains("/api/test"));
    }
}
