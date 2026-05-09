use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Curl,
    Fetch,
    Python,
    Go,
}

#[tauri::command]
pub fn generate_code_snippet(
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: String,
    format: ExportFormat,
) -> Result<String, String> {
    match format {
        ExportFormat::Curl => Ok(generate_curl(&method, &url, &headers, &body)),
        ExportFormat::Fetch => Ok(generate_fetch(&method, &url, &headers, &body)),
        ExportFormat::Python => Ok(generate_python(&method, &url, &headers, &body)),
        ExportFormat::Go => Ok(generate_go(&method, &url, &headers, &body)),
    }
}

fn generate_curl(method: &str, url: &str, headers: &HashMap<String, String>, body: &str) -> String {
    let mut parts = vec!["curl".to_string()];
    if method != "GET" && method != "POST" {
        parts.push(format!("-X {}", method));
    }
    for (k, v) in headers {
        if !k.to_lowercase().contains("host") {
            parts.push(format!("-H '{}: {}'", k, v));
        }
    }
    if !body.is_empty() {
        parts.push(format!("-d '{}'", body));
    }
    parts.push(format!("'{}'", url));
    parts.join(" \\\n  ")
}

fn generate_fetch(method: &str, url: &str, headers: &HashMap<String, String>, body: &str) -> String {
    let mut lines = vec!["fetch('".to_string() + url + "', {"];
    lines.push(format!("  method: '{}',", method));
    if !headers.is_empty() {
        lines.push("  headers: {".to_string());
        for (k, v) in headers {
            if !k.to_lowercase().contains("host") {
                lines.push(format!("    '{}': '{}',", k, v));
            }
        }
        lines.push("  },".to_string());
    }
    if !body.is_empty() {
        let escaped = body.replace('\'', "\\'");
        lines.push(format!("  body: '{}',", escaped));
    }
    lines.push("})".to_string());
    lines.push("  .then(res => res.json())".to_string());
    lines.push("  .then(data => console.log(data));".to_string());
    lines.join("\n")
}

fn generate_python(method: &str, url: &str, headers: &HashMap<String, String>, body: &str) -> String {
    let mut lines = vec!["import requests".to_string(), "".to_string()];
    lines.push("response = requests.request(".to_string());
    lines.push(format!("    method='{}',", method));
    lines.push(format!("    url='{}',", url));
    if !headers.is_empty() {
        lines.push("    headers={".to_string());
        for (k, v) in headers {
            if !k.to_lowercase().contains("host") {
                lines.push(format!("        '{}': '{}',", k, v));
            }
        }
        lines.push("    },".to_string());
    }
    if !body.is_empty() {
        lines.push(format!("    data='{}',", body));
    }
    lines.push(")".to_string());
    lines.push("".to_string());
    lines.push("print(response.status_code)".to_string());
    lines.push("print(response.json())".to_string());
    lines.join("\n")
}

fn generate_go(method: &str, url: &str, headers: &HashMap<String, String>, body: &str) -> String {
    let mut lines = vec![
        "package main".to_string(),
        "".to_string(),
        "import (".to_string(),
        "    \"fmt\"".to_string(),
        "    \"io\"".to_string(),
        "    \"net/http\"".to_string(),
        "    \"strings\"".to_string(),
        ")".to_string(),
        "".to_string(),
        "func main() {".to_string(),
    ];
    if !body.is_empty() {
        lines.push(format!("    body := strings.NewReader(`{}`)", body));
        lines.push("    req, _ := http.NewRequest(\"".to_string() + method + "\", \"" + url + "\", body)");
    } else {
        lines.push("    req, _ := http.NewRequest(\"".to_string() + method + "\", \"" + url + "\", nil)");
    }
    for (k, v) in headers {
        if !k.to_lowercase().contains("host") {
            lines.push(format!("    req.Header.Set(\"{}\", \"{}\")", k, v));
        }
    }
    lines.push("".to_string());
    lines.push("    client := &http.Client{}".to_string());
    lines.push("    resp, _ := client.Do(req)".to_string());
    lines.push("    defer resp.Body.Close()".to_string());
    lines.push("".to_string());
    lines.push("    data, _ := io.ReadAll(resp.Body)".to_string());
    lines.push("    fmt.Println(string(data))".to_string());
    lines.push("}".to_string());
    lines.join("\n")
}
