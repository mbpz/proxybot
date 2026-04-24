//! React scaffold generator for ProxyBot.

use crate::db::DbState;
use crate::infer::{ApiInterface, InferredApi};
use crate::replay::compute_diff;
use crate::vision::{VisionComponent, ComponentTree};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::process::{Command, Child};
use tokio::time::{sleep, Duration};
use reqwest::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaffoldComponent {
    pub name: String,
    pub route_path: String,
    pub file_path: String,
    pub content: String,
    /// Rendered vision component tree TSX for this page (when vision is provided).
    pub vision_tree: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaffoldStore {
    pub module_name: String,
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaffoldTest {
    pub name: String,
    pub route_path: String,
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaffoldProject {
    pub name: String,
    pub base_path: String,
    pub components: Vec<ScaffoldComponent>,
    pub stores: Vec<ScaffoldStore>,
    pub tests: Vec<ScaffoldTest>,
    pub files: HashMap<String, String>,
}

#[allow(dead_code)]
fn get_anthropic_api_key() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("CLAUDE_API_KEY"))
        .ok()
}

#[allow(dead_code)]
async fn call_claude_api(prompt: &str, api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-7-20251101",
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
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

    let api_resp: ApiResponse =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse: {}", e))?;

    for block in api_resp.content {
        if block.block_type == "text" {
            if let Some(text) = block.text {
                return Ok(text);
            }
        }
    }
    Err("No text content".to_string())
}

fn infer_route(path: &str) -> String {
    path.to_string()
}

fn group_by_prefix(apis: &[InferredApi]) -> HashMap<String, Vec<usize>> {
    let mut map: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, api) in apis.iter().enumerate() {
        let part = api.path.split('/').filter(|s| !s.is_empty()).nth(0).unwrap_or("api");
        map.entry(part.to_string()).or_default().push(i);
    }
    map
}

fn hook_name(n: &str) -> String {
    let c = n.replace("/", "").replace("_", "").replace("-", "");
    if c.is_empty() { return "useHook".to_string(); }
    format!("use{}{}", c.chars().next().unwrap().to_uppercase(), &c[1..])
}

fn page_name(n: &str) -> String {
    format!("{}Page", hook_name(n).replace("use", ""))
}

fn pkg_json(name: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "name": name, "private": true, "version": "0.0.1", "type": "module",
        "scripts": {"dev": "vite", "build": "tsc && vite build", "preview": "vite preview", "test:e2e": "playwright test"},
        "dependencies": {"react": "^18.2.0", "react-dom": "^18.2.0", "react-router-dom": "^6.20.0", "zustand": "^4.4.7", "axios": "^1.6.2"},
        "devDependencies": {"@playwright/test": "^1.40.0", "@types/react": "^18.2.43", "@types/react-dom": "^18.2.17", "@vitejs/plugin-react": "^4.2.1", "typescript": "^5.3.2", "vite": "^5.0.8"}
    })).unwrap()
}

fn vite_config() -> String {
    r#"import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
export default defineConfig({ plugins: [react()], server: { port: 3000, proxy: { '/api': { target: 'http://localhost:8000', changeOrigin: true } } } })
"#.to_string()
}

fn tsconfig() -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "compilerOptions": {"target": "ES2020", "useDefineForClassFields": true, "lib": ["ES2020", "DOM", "DOM.Iterable"], "module": "ESNext", "skipLibCheck": true, "moduleResolution": "bundler", "allowImportingTsExtensions": true, "resolveJsonModule": true, "isolatedModules": true, "noEmit": true, "jsx": "react-jsx", "strict": true, "noUnusedLocals": true, "noUnusedParameters": true, "noFallthroughCasesInSwitch": true},
        "include": ["src"], "references": [{ "path": "./tsconfig.node.json" }]
    })).unwrap()
}

fn tsconfig_node() -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "compilerOptions": {"composite": true, "skipLibCheck": true, "module": "ESNext", "moduleResolution": "bundler", "allowSyntheticDefaultImports": true},
        "include": ["vite.config.ts"]
    })).unwrap()
}

fn index_html() -> String {
    r#"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"/><meta name="viewport" content="width=device-width, initial-scale=1.0"/><title>ProxyBot Scaffold</title></head><body><div id="root"></div><script type="module" src="/src/main.tsx"></script></body></html>"#.to_string()
}

fn main_tsx() -> String {
    r#"import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'
ReactDOM.createRoot(document.getElementById('root')!).render(<React.StrictMode><App /></React.StrictMode>)
"#.to_string()
}

fn css() -> String {
    r#"*{margin:0;padding:0;box-sizing:border-box}body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Oxygen,Ubuntu,sans-serif;line-height:1.5;background:#f5f5f5;color:#333}#root{min-height:100vh}.container{max-width:1200px;margin:0 auto;padding:2rem}.header{text-align:center;margin-bottom:2rem}.page{background:white;border-radius:8px;padding:1.5rem;box-shadow:0 1px 3px rgba(0,0,0,0.1)}.page h1{margin-bottom:1rem;color:#111}.loading,.error{text-align:center;padding:2rem}.error{color:#dc2626}.btn{padding:0.5rem 1rem;border:none;border-radius:4px;cursor:pointer;font-size:0.875rem}.btn-primary{background:#2563eb;color:white}.btn-primary:hover{background:#1d4ed8}
"#.to_string()
}

fn app_tsx(routes: &[(String, String)]) -> String {
    let mut imp = String::new();
    let mut els = String::new();
    for (cn, rp) in routes {
        let p = format!("./pages/{}", cn.replace("Page", "").to_lowercase());
        imp.push_str(&format!("import {} from '{}';\n", cn, p));
        els.push_str(&format!("      <Route path=\"{}\" element={{<{} />}} />\n", rp, cn));
    }
    format!(r#"import {{BrowserRouter,Routes,Route}} from'react-router-dom'
import'./index.css'
{}
function App(){{return(<BrowserRouter><div className="container"><header className="header"><h1>ProxyBot Scaffold</h1></header><Routes>
{}
</Routes></div></BrowserRouter>)}}
export default App
"#, imp, els)
}

fn hook_get(hn: &str, rr: &str, path: &str, auth_dep: &str) -> String {
    format!(
        "export function {hn}(o?){{const[d,sD]=React.useState<{rr}|null>(null);const[l,sL]=React.useState(false);const[e,sE]=React.useState(null);React.useEffect(()=>{{if(o?.enabled===false)return;async function f(){{sL(true);try{{const r=await fetch('/api{path}');if(!r.ok)throw new Error('fail');sD(await r.json())}}catch(e){{sE(e instanceof Error?e.message:'err')}}finally{{sL(false)}}}}f()}},[{auth_dep}]);return{{data:d,loading:l,error:e}}}}\n",
        hn = hn, rr = rr, path = path, auth_dep = auth_dep
    )
}

fn hook_mutation(hn: &str, path: &str, method: &str, auth_header: &str) -> String {
    let extra_headers = if auth_header.is_empty() {
        "'Content-Type':'application/json'".to_string()
    } else {
        format!("'Content-Type':'application/json',{}", auth_header)
    };
    let mut s = String::new();
    s.push_str("export function ");
    s.push_str(hn);
    s.push_str("(o?)");
    s.push_str("{const[l,sL]=React.useState(false);const[e,sE]=React.useState(null);async function m(b)");
    s.push_str("{sL(true);try");
    s.push_str("{const r=await fetch('/api");
    s.push_str(path);
    s.push_str("',{method:'");
    s.push_str(method);
    s.push_str("',headers:{");
    s.push_str(&extra_headers);
    s.push_str("}})}");
    s.push_str("catch(e){const m=e instanceof Error?e.message:'err';sE(m);o?.onError?.(m);throw e}");
    s.push_str("finally{sL(false)}}");
    s.push_str("return{mutate:m,loading:l,error:e}}");
    s.push('\n');
    s
}

fn hook(iface: &ApiInterface, module: &str) -> String {
    let hn = hook_name(&iface.name);
    let rr = format!("{}Api.{}Response", module, iface.name);
    if iface.method.to_uppercase() == "GET" {
        let auth_dep = if iface.auth_required { "localStorage.getItem('authToken')" } else { "null" };
        hook_get(&hn, &rr, &iface.path, auth_dep)
    } else {
        let auth_header: String = if iface.auth_required { "Authorization:`Bearer ${localStorage.getItem('authToken')}`,".to_string() } else { String::new() };
        hook_mutation(&hn, &iface.path, &iface.method.to_uppercase(), &auth_header)
    }
}

fn page_get(hn: &str, cn: &str, name: &str) -> String {
    format!(
        "import React from 'react'\nimport {{h}} from '../hooks/hooks'\nexport default function {cn}(){{const {{data,loading,error}}}}={hn}()\nif(loading)return<div className=\"loading\">Loading...</div>\nif(error)return<div className=\"error\">Error:{{error}}</div>\nreturn(<div className=\"page\"><h1>{name}</h1><pre>{{JSON.stringify(data,null,2)}}</pre></div>)}}\n",
        hn = hn, cn = cn, name = name
    )
}

fn page_mutation(hn: &str, cn: &str, name: &str) -> String {
    format!(
        "import React,{{useState}}from'react'\nimport{{h}}from'../hooks/hooks'\nexport default function {cn}(){{const {{mutate,loading,error}}}}={hn}()\nconst[r,sR]=useState(null)\nconst handleSubmit=async(e)=>{{e.preventDefault();try{{const d=await mutate({{}});sR(d)}}catch{{}}}}\nreturn(<div className=\"page\"><h1>{name}</h1><form onSubmit={{handleSubmit}}><button type=\"submit\"className=\"btn btn-primary\"disabled={{loading}}>{{loading?'Submitting...':'Submit'}}</button></form>\n{{error&&<div className=\"error\">{{error}}</div>}}\n{{r&&<pre>{{JSON.stringify(r,null,2)}}</pre>}}</div>)}}\n",
        hn = hn, cn = cn, name = name
    )
}

fn page(iface: &ApiInterface, _module: &str) -> String {
    let hn = hook_name(&iface.name);
    let cn = page_name(&iface.name);
    if iface.method.to_uppercase() == "GET" {
        page_get(&hn, &cn, &iface.name)
    } else {
        page_mutation(&hn, &cn, &iface.name)
    }
}

// ============================================================================
// Vision → React Component Rendering
// ============================================================================

/// Build inline style string from VisionPosition.
fn vision_style(pos: &crate::vision::VisionPosition) -> String {
    format!(
        "position:absolute;left:{}%;top:{}%;width:{}%;height:{}%",
        (pos.x / 10.0).min(100.0),
        (pos.y / 10.0).min(100.0),
        (pos.width / 10.0).min(100.0),
        (pos.height / 10.0).min(100.0)
    )
}

/// Render a VisionComponent as a React TSX string.
fn vision_element(vc: &VisionComponent, api_method: &str, _api_name: &str, hook_name_str: &str) -> String {
    let ctype = vc.component_type.to_lowercase();
    let text = vc.text.as_deref().unwrap_or("");
    let style = vision_style(&vc.position);

    // Render children first
    let children_rendering: Vec<String> = vc.children
        .iter()
        .map(|child| vision_element(child, api_method, _api_name, hook_name_str))
        .collect();

    match ctype.as_str() {
        "button" => {
            let label = if text.is_empty() { "Button" } else { text };
            if api_method != "GET" {
                // Mutation button: calls mutate on click
                format!(
                    "<button className=\"btn btn-primary\" style={{{}}} onClick={{{{() => mutate && mutate({{}})}}}}>{}</button>",
                    style, label
                )
            } else {
                format!(
                    "<button className=\"btn btn-primary\" style={{{}}}>{{data ? 'Done' : 'Loading...'}}</button>",
                    style
                )
            }
        }
        "text" | "label" => {
            if api_method == "GET" {
                format!(
                    "<span className=\"vision-text\" style={{{}}}>{{data ? JSON.stringify(data) : '{}'}}</span>",
                    style,
                    text.replace('\'', "\\'")
                )
            } else {
                format!(
                    "<span className=\"vision-text\" style={{{}}}>{}</span>",
                    style, text
                )
            }
        }
        "input" | "textinput" | "text_field" => {
            format!(
                "<input className=\"vision-input\" type=\"text\" placeholder=\"{}\" style={{{}}} />",
                text.replace('\'', "\\'"),
                style
            )
        }
        "image" | "img" => {
            format!(
                "<img className=\"vision-image\" alt=\"{}\" style={{{}}} />",
                text.replace('\'', "\\'"),
                style
            )
        }
        "card" | "container" | "view" | "header" | "nav" | "list" | "table" => {
            let cls = format!("vision-{}", ctype);
            if children_rendering.is_empty() {
                format!(
                    "<div className=\"{}\" style={{{}}}>{}</div>",
                    cls, style, text
                )
            } else {
                format!(
                    "<div className=\"{}\" style={{{}}}>\n{}\n</div>",
                    cls, style,
                    children_rendering.join("\n")
                )
            }
        }
        "listitem" | "row" => {
            if children_rendering.is_empty() {
                format!("<div className=\"vision-list-item\" style={{{}}}>{}</div>", style, text)
            } else {
                format!(
                    "<div className=\"vision-list-item\" style={{{}}}>\n{}\n</div>",
                    style,
                    children_rendering.join("\n")
                )
            }
        }
        _ => {
            if children_rendering.is_empty() {
                format!(
                    "<div className=\"vision-{}\" style={{{}}}>{}</div>",
                    ctype, style,
                    text.replace('\'', "\\'")
                )
            } else {
                format!(
                    "<div className=\"vision-{}\" style={{{}}}>\n{}\n</div>",
                    ctype, style,
                    children_rendering.join("\n")
                )
            }
        }
    }
}

/// Render a list of vision components as a complete React TSX page component.
fn render_vision_page(
    page_name_str: &str,
    api_method: &str,
    _api_name: &str,
    components: &[VisionComponent],
    hook_name_str: &str,
) -> String {
    let hook_call = if api_method == "GET" {
        format!("const {{data, loading, error}} = {hook_name_str}()")
    } else {
        format!("const {{mutate, loading, error}} = {hook_name_str}()")
    };

    let elements_str = components
        .iter()
        .map(|vc| vision_element(vc, api_method, _api_name, hook_name_str))
        .collect::<Vec<_>>()
        .join("\n");

    // Build result using String concatenation to avoid escape issues with JSX braces
    let mut result = String::new();
    result.push_str("import React from 'react';\n");
    result.push_str("import {useParams} from 'react-router-dom';\n");
    result.push_str(&format!("import {{use{}}} from '../hooks/hooks';\n\n", hook_name_str));
    result.push_str("interface Props {}\n\n");
    result.push_str(&format!("export default function {page_name_str}Page() {{\n"));
    result.push_str(&format!("  {}\n", hook_call));
    result.push_str("  if (loading) return <div className=\"loading\">Loading...</div>;\n");
    result.push_str("  if (error) return <div className=\"error\">Error: {String(error)}</div>;\n");
    result.push_str("  return (\n");
    result.push_str("    <div className=\"page vision-page\" style={{position: 'relative', width: '100%', minHeight: '100vh'}}>\n");
    result.push_str("      ");
    result.push_str(&elements_str);
    result.push_str("\n    </div>\n");
    result.push_str("  )\n");
    result.push_str("}\n");
    result
}

// ============================================================================
// Store / Route helpers
// ============================================================================

fn store(module: &str, apis: &[&InferredApi]) -> String {
    let sn = format!("use{}{}Store", module.chars().next().unwrap().to_uppercase(), &module[1..]);
    let mut sf = String::new();
    let mut act = String::new();
    for a in apis {
        let fn2 = a.name.replace("/", "_").replace("-", "_").to_lowercase();
        let rt = a.name.replace("/", "").replace("-", "");
        sf.push_str(&format!("  {}:{}Response|null;\n", fn2, rt));
        act.push_str(&format!("  s{}:(d:{}Response)=>void;\n", rt, rt));
    }
    format!(
        "import{{create}}from'zustand'\ninterface {{S}}{fields}\ninterface {{A}}{actions}\nexport const {s}=create<{{S}}&{{A}}>((s)=>({{\n//Actions\n}}))\n",
        s = sn, fields = sf.trim(), actions = act.trim()
    )
}

fn pw_config() -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "testDir": "./e2e", "timeout": 30000,
        "use": {"baseURL": "http://localhost:3000", "trace": "on-first-retry"},
        "projects": [{"name": "chromium", "use": {"browserName": "chromium"}}]
    })).unwrap()
}

fn pw_test(rp: &str, cn: &str) -> String {
    format!(r#"import{{test,expect}}from'@playwright/test'
test('{} loads',async{{page}})=>{{await page.goto('{}');await page.waitForLoadState('networkidle');await expect(page.locator('.error')).not.toBeVisible()}})
test('{} shows content',async{{page}})=>{{await page.goto('{}');await page.waitForLoadState('networkidle');await expect(page.locator('.page')).toBeVisible()}})
"#, cn, rp, cn, rp)
}

fn get_apis(conn: &rusqlite::Connection, sid: &str) -> Result<Vec<InferredApi>, String> {
    let mut s = conn.prepare("SELECT id,session_id,name,method,path,params,auth_required,request_ids,score,created_at FROM inferred_apis WHERE session_id=?1 ORDER BY id").map_err(|e| e.to_string())?;
    let rows = s.query_map(params![sid], |row| {
        Ok(InferredApi { id: row.get(0)?, session_id: row.get(1)?, name: row.get(2)?, method: row.get(3)?, path: row.get(4)?, params: row.get(5)?, auth_required: row.get::<_, i32>(6)? != 0, request_ids: row.get(7)?, score: row.get(8)?, created_at: row.get(9)? })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn generate_scaffold_project(db: State<'_, Arc<DbState>>, session_id: String, name: Option<String>) -> Result<ScaffoldProject, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let apis = get_apis(&conn, &session_id)?;
    if apis.is_empty() { return Err("No inferred APIs. Run inference first.".to_string()); }
    let n = name.unwrap_or_else(|| "proxybot_frontend".to_string());
    let map = group_by_prefix(&apis);
    let mut comps = Vec::new();
    let mut stores = Vec::new();
    let mut files = HashMap::new();
    let mut routes = Vec::new();
    for (mn, idx) in &map {
        let mas: Vec<&InferredApi> = idx.iter().filter_map(|&i| apis.get(i)).collect();
        for a in &mas {
            let ir = ApiInterface { name: a.name.clone(), method: a.method.clone(), path: a.path.clone(), params: a.params.clone(), auth_required: a.auth_required };
            let h = hook(&ir, mn);
            files.insert(format!("src/hooks/{}Hooks.tsx", mn), h);
            let pc = page(&ir, mn);
            let cn = page_name(&a.name);
            let pf = format!("src/pages/{}.tsx", cn);
            files.insert(pf.clone(), pc.clone());
            comps.push(ScaffoldComponent { name: cn.clone(), route_path: infer_route(&a.path), file_path: pf, content: pc, vision_tree: None });
            routes.push((cn, infer_route(&a.path)));
        }
        let mirs: Vec<&InferredApi> = mas.iter().map(|&a| a).collect();
        let sc = store(mn, &mirs);
        stores.push(ScaffoldStore { module_name: mn.clone(), file_path: format!("src/stores/{}Store.ts", mn), content: sc.clone() });
        files.insert(format!("src/stores/{}Store.ts", mn), sc);
    }
    files.insert("package.json".to_string(), pkg_json(&n));
    files.insert("vite.config.ts".to_string(), vite_config());
    files.insert("tsconfig.json".to_string(), tsconfig());
    files.insert("tsconfig.node.json".to_string(), tsconfig_node());
    files.insert("index.html".to_string(), index_html());
    files.insert("src/main.tsx".to_string(), main_tsx());
    files.insert("src/index.css".to_string(), css());
    files.insert("src/App.tsx".to_string(), app_tsx(&routes));
    files.insert("playwright.config.ts".to_string(), pw_config());
    let mut tests = Vec::new();
    for c in &comps {
        let tc = pw_test(&c.route_path, &c.name);
        let tf = format!("e2e/{}.spec.ts", c.name.replace("Page", "").to_lowercase());
        files.insert(tf.clone(), tc.clone());
        tests.push(ScaffoldTest { name: c.name.clone(), route_path: c.route_path.clone(), file_path: tf, content: tc });
    }
    Ok(ScaffoldProject { name: n, base_path: String::new(), components: comps, stores, tests, files })
}

/// Generate scaffold with vision-enhanced pages.
/// When `vision` is Some, each page is rendered using the vision component tree
/// instead of the generic JSON-data page. Components are positioned using
/// absolute positioning based on their VisionPosition.
#[tauri::command]
pub fn generate_scaffold_with_vision(
    db: State<'_, Arc<DbState>>,
    session_id: String,
    name: Option<String>,
    vision_json: Option<String>,
) -> Result<ScaffoldProject, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let apis = get_apis(&conn, &session_id)?;
    if apis.is_empty() {
        return Err("No inferred APIs. Run inference first.".to_string());
    }

    // Parse optional vision ComponentTree
    let vision_tree: Option<ComponentTree> = vision_json
        .as_ref()
        .and_then(|j| serde_json::from_str::<ComponentTree>(j).ok());

    let n = name.unwrap_or_else(|| "proxybot_frontend".to_string());
    let map = group_by_prefix(&apis);
    let mut comps = Vec::new();
    let mut stores = Vec::new();
    let mut files: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut routes = Vec::new();

    for (mn, idx) in &map {
        let mas: Vec<&InferredApi> = idx.iter().filter_map(|i| apis.get(*i)).collect();

        for a in &mas {
            let ir = ApiInterface {
                name: a.name.clone(),
                method: a.method.clone(),
                path: a.path.clone(),
                params: a.params.clone(),
                auth_required: a.auth_required,
            };
            let h = hook(&ir, mn);
            files.insert(format!("src/hooks/{}Hooks.tsx", mn), h);

            let cn = page_name(&a.name);
            let pf = format!("src/pages/{}.tsx", cn);

            // Decide page content: vision-enhanced or generic
            let (pc, vision_tree_str) = if let Some(ref vt) = vision_tree {
                // If vision data has suggested routes, use all components for all pages
                // (the complete component tree represents the full app UI)
                if vt.suggested_routes.is_empty() {
                    (page(&ir, mn), None)
                } else {
                    // Render this page using the full vision component tree
                    let hn = hook_name(&a.name);
                    let vp = render_vision_page(&cn, &a.method, &a.name, &vt.components, &hn);
                    (vp.clone(), Some(vp))
                }
            } else {
                (page(&ir, mn), None)
            };

            files.insert(pf.clone(), pc.clone());
            comps.push(ScaffoldComponent {
                name: cn.clone(),
                route_path: infer_route(&a.path),
                file_path: pf,
                content: pc,
                vision_tree: vision_tree_str,
            });
            routes.push((cn, infer_route(&a.path)));
        }

        let mirs: Vec<&InferredApi> = mas.iter().map(|a| *a).collect();
        let sc = store(mn, &mirs);
        stores.push(ScaffoldStore {
            module_name: mn.clone(),
            file_path: format!("src/stores/{}Store.ts", mn),
            content: sc.clone(),
        });
        files.insert(format!("src/stores/{}Store.ts", mn), sc);
    }

    let mut all_files = files;
    all_files.insert("package.json".to_string(), pkg_json(&n));
    all_files.insert("vite.config.ts".to_string(), vite_config());
    all_files.insert("tsconfig.json".to_string(), tsconfig());
    all_files.insert("tsconfig.node.json".to_string(), tsconfig_node());
    all_files.insert("index.html".to_string(), index_html());
    all_files.insert("src/main.tsx".to_string(), main_tsx());
    all_files.insert("src/index.css".to_string(), css());
    all_files.insert("src/App.tsx".to_string(), app_tsx(&routes));
    all_files.insert("playwright.config.ts".to_string(), pw_config());

    let mut tests = Vec::new();
    for c in &comps {
        let tc = pw_test(&c.route_path, &c.name);
        let tf = format!("e2e/{}.spec.ts", c.name.replace("Page", "").to_lowercase());
        all_files.insert(tf.clone(), tc.clone());
        tests.push(ScaffoldTest {
            name: c.name.clone(),
            route_path: c.route_path.clone(),
            file_path: tf,
            content: tc,
        });
    }

    Ok(ScaffoldProject {
        name: n,
        base_path: String::new(),
        components: comps,
        stores,
        tests,
        files: all_files,
    })
}

#[tauri::command]
pub fn write_scaffold_project(db: State<'_, Arc<DbState>>, session_id: String, name: Option<String>, dir: Option<String>) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let apis = get_apis(&conn, &session_id)?;
    if apis.is_empty() { return Err("No inferred APIs. Run inference first.".to_string()); }
    let n = name.unwrap_or_else(|| "proxybot_frontend".to_string());
    let base = dir.unwrap_or_else(|| format!("{}/.proxybot/scaffold_projects/{}", std::env::var("HOME").unwrap_or_else(|_| ".".to_string()), n));
    let bp = PathBuf::from(&base);
    let src = bp.join("src");
    let pages = src.join("pages");
    let hooks = src.join("hooks");
    let stores = src.join("stores");
    let e2e = bp.join("e2e");
    fs::create_dir_all(&pages).map_err(|e| e.to_string())?;
    fs::create_dir_all(&hooks).map_err(|e| e.to_string())?;
    fs::create_dir_all(&stores).map_err(|e| e.to_string())?;
    fs::create_dir_all(&e2e).map_err(|e| e.to_string())?;
    let map = group_by_prefix(&apis);
    let mut routes = Vec::new();
    for (mn, idx) in &map {
        let mas: Vec<&InferredApi> = idx.iter().filter_map(|&i| apis.get(i)).collect();
        for a in &mas {
            let ir = ApiInterface { name: a.name.clone(), method: a.method.clone(), path: a.path.clone(), params: a.params.clone(), auth_required: a.auth_required };
            fs::write(hooks.join(format!("{}Hooks.tsx", mn)), hook(&ir, mn)).map_err(|e| e.to_string())?;
            let pc = page(&ir, mn);
            let cn = page_name(&a.name);
            fs::write(pages.join(format!("{}.tsx", cn)), &pc).map_err(|e| e.to_string())?;
            routes.push((cn, infer_route(&a.path)));
        }
        let mirs: Vec<&InferredApi> = mas.iter().map(|&a| a).collect();
        fs::write(stores.join(format!("{}Store.ts", mn)), store(mn, &mirs)).map_err(|e| e.to_string())?;
    }
    fs::write(bp.join("package.json"), pkg_json(&n)).map_err(|e| e.to_string())?;
    fs::write(bp.join("vite.config.ts"), vite_config()).map_err(|e| e.to_string())?;
    fs::write(bp.join("tsconfig.json"), tsconfig()).map_err(|e| e.to_string())?;
    fs::write(bp.join("tsconfig.node.json"), tsconfig_node()).map_err(|e| e.to_string())?;
    fs::write(bp.join("index.html"), index_html()).map_err(|e| e.to_string())?;
    fs::write(src.join("main.tsx"), main_tsx()).map_err(|e| e.to_string())?;
    fs::write(src.join("index.css"), css()).map_err(|e| e.to_string())?;
    fs::write(src.join("App.tsx"), app_tsx(&routes)).map_err(|e| e.to_string())?;
    fs::write(bp.join("playwright.config.ts"), pw_config()).map_err(|e| e.to_string())?;
    for (cn, rp) in &routes {
        fs::write(e2e.join(format!("{}.spec.ts", cn.replace("Page", "").to_lowercase())), pw_test(rp, cn)).map_err(|e| e.to_string())?;
    }
    log::info!("Scaffold written to {}", base);
    Ok(base)
}

/// Write a pre-generated scaffold project (with optional vision pages) to disk.
#[tauri::command]
pub fn write_scaffold_project_with_vision(
    project: ScaffoldProject,
    output_dir: Option<String>,
) -> Result<String, String> {
    let base = output_dir.unwrap_or_else(|| {
        format!(
            "{}/.proxybot/scaffold_projects/{}",
            std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
            project.name
        )
    });
    let bp = PathBuf::from(&base);
    fs::create_dir_all(&bp).map_err(|e| e.to_string())?;

    // Write all files from the project
    for (path, content) in &project.files {
        let file_path = bp.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&file_path, content).map_err(|e| e.to_string())?;
    }

    log::info!("Vision scaffold written to {}", base);
    Ok(base)
}

// ============================================================================
// Evaluation: Recorded HTTP exchange
// ============================================================================

/// A recorded HTTP exchange for evaluation.
#[derive(Debug, Clone)]
struct RecordedExchange {
    method: String,
    path: String,
    req_body: Option<String>,
    resp_status: u16,
    resp_headers: Vec<(String, String)>,
    resp_body: Option<String>,
}

/// Start a background process and return its handle.
async fn start_background_process(program: &str, args: &[&str], cwd: &str) -> Result<Child, String> {
    let child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", program, e))?;
    Ok(child)
}

/// Wait for a server to be ready by polling until HTTP 200.
async fn wait_for_server(url: &str, max_wait_secs: u64) -> Result<(), String> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;
    let deadline = std::time::Instant::now() + Duration::from_secs(max_wait_secs);
    while std::time::Instant::now() < deadline {
        if client.get(url).send().await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }
    Err(format!("Server {} did not become ready in {}s", url, max_wait_secs))
}

// ============================================================================
// Evaluation: HTTP replay + diff
// ============================================================================

/// Replay recorded requests against a running mock API and compute diffs.
async fn replay_and_diff(
    mock_api_url: &str,
    exchanges: &[RecordedExchange],
) -> (usize, usize, Vec<String>) {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_else(|_| Client::new());
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();

    for ex in exchanges {
        let url = format!("{}{}", mock_api_url.trim_end_matches('/'), ex.path);
        let req_builder = client.request(
            reqwest::Method::from_bytes(ex.method.as_bytes()).unwrap_or(reqwest::Method::GET),
            &url,
        );
        let req_builder = if let Some(ref body) = ex.req_body {
            req_builder.body(body.clone())
        } else {
            req_builder
        };
        // Add auth headers if present
        let req_builder = req_builder.header("Accept", "application/json");

        match req_builder.send().await {
            Ok(resp) => {
                let mock_status = resp.status().as_u16();
                let mock_headers: Vec<(String, String)> = resp
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                let mock_body_text = resp.text().await.ok();

                let diff = compute_diff(
                    &ex.resp_status,
                    &ex.resp_headers,
                    &ex.resp_body,
                    &mock_status,
                    &mock_headers,
                    &mock_body_text,
                );

                if !diff.has_changes {
                    passed += 1;
                } else {
                    failed += 1;
                    let header_errs: Vec<String> = diff
                        .header_diffs
                        .iter()
                        .filter(|d| d.diff_type != crate::replay::DiffType::Unchanged)
                        .map(|d| format!("header {}: recorded={:?} mock={:?}", d.header, d.recorded, d.mock))
                        .collect();
                    let err_msg = format!("{} {} → {} (status {})",
                        ex.method, ex.path,
                        if header_errs.is_empty() { "OK".to_string() } else { header_errs.join(", ") },
                        mock_status
                    );
                    errors.push(err_msg);
                }
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("{} {} → network error: {}", ex.method, ex.path, e));
            }
        }
    }

    (passed, failed, errors)
}

/// Run Playwright tests and parse JSON results.
async fn run_playwright_tests(scaffold_path: &str) -> (usize, usize, Vec<String>) {
    // Run `npx playwright test --reporter=json` in the scaffold dir
    let output = Command::new("npx")
        .args(["playwright", "test", "--reporter=json"])
        .current_dir(scaffold_path)
        .output()
        .await
        .map_err(|e| format!("Failed to run playwright: {}", e))
        .ok();

    match output {
        Some(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Try to parse Playwright JSON output
            // Playwright JSON reporter outputs array of test results
            #[derive(Deserialize)]
            struct PwResult {
                title: String,
                status: String,
                errors: Vec<String>,
            }
            match serde_json::from_str::<Vec<PwResult>>(&stdout) {
                Ok(results) => {
                    let passed = results.iter().filter(|r| r.status == "passed").count();
                    let failed = results.iter().filter(|r| r.status != "passed").count();
                    let errors: Vec<String> = results
                        .iter()
                        .filter(|r| r.status != "passed")
                        .flat_map(|r| r.errors.iter().cloned().map(|e| format!("{}: {}", r.title, e)))
                        .collect();
                    (passed, failed, errors)
                }
                Err(_) => {
                    // Fallback: count by searching for pass/fail in output
                    let pass_count = stdout.matches("passed").count();
                    let fail_count = stdout.matches("failed").count();
                    (pass_count, fail_count, vec![])
                }
            }
        }
        None => (0, 0, vec!["Playwright not available".to_string()]),
    }
}

// ============================================================================
// Main evaluation logic
// ============================================================================

/// Evaluate scaffold by starting servers and running real tests.
async fn eval_scaffold(scaffold_path: &str, exchanges: Vec<RecordedExchange>) -> Result<(bool, f64, Vec<String>), String> {
    let scaffold_pathbuf = PathBuf::from(scaffold_path);
    let mock_api_path = scaffold_pathbuf.join("mock-api");
    let has_mock_api = mock_api_path.join("main.py").exists();

    // 1. Start mock API on port 8000 (if available)
    let mut mock_child: Option<Child> = None;
    if has_mock_api {
        // Install deps and start uvicorn
        let _ = Command::new("pip")
            .args(["install", "-q", "-r", "requirements.txt"])
            .current_dir(&mock_api_path)
            .output()
            .await;
        match start_background_process("uvicorn", &["main:app", "--host", "0.0.0.0", "--port", "8000"], mock_api_path.to_str().unwrap_or(".")).await {
            Ok(child) => { mock_child = Some(child); }
            Err(e) => { log::warn!("Could not start mock API: {}", e); }
        }
    }

    // 2. Start frontend dev server on port 3000
    let mut frontend_child: Option<Child> = None;
    let vite_configured = scaffold_pathbuf.join("vite.config.ts").exists();
    if vite_configured {
        // Install deps first
        let _ = Command::new("npm")
            .args(["install", "--silent"])
            .current_dir(&scaffold_pathbuf)
            .output()
            .await;
        match start_background_process("npm", &["run", "dev", "--", "--port", "3000", "--host"], scaffold_path).await {
            Ok(child) => { frontend_child = Some(child); }
            Err(e) => { log::warn!("Could not start frontend: {}", e); }
        }
    }

    let mut all_errors = Vec::new();
    let mut total_passed = 0usize;
    let mut total_failed = 0usize;

    // 3. If mock API is running, replay and diff
    if mock_child.is_some() && !exchanges.is_empty() {
        if wait_for_server("http://localhost:8000", 15).await.is_ok() {
            let (passed, failed, diff_errors) = replay_and_diff("http://localhost:8000", &exchanges).await;
            total_passed += passed;
            total_failed += failed;
            all_errors.extend(diff_errors);
        } else {
            all_errors.push("Mock API did not start".to_string());
        }
    }

    // 4. Run Playwright tests
    if frontend_child.is_some() && vite_configured {
        if wait_for_server("http://localhost:3000", 30).await.is_ok() {
            let (pw_pass, pw_fail, pw_errors) = run_playwright_tests(scaffold_path).await;
            total_passed += pw_pass;
            total_failed += pw_fail;
            all_errors.extend(pw_errors);
        } else {
            all_errors.push("Frontend dev server did not start".to_string());
        }
    }

    // 5. Cleanup: kill servers
    if let Some(mut child) = mock_child {
        let _ = child.kill().await;
    }
    if let Some(mut child) = frontend_child {
        let _ = child.kill().await;
    }

    // 6. Compute score and result
    let total = total_passed + total_failed;
    let score = if total > 0 {
        total_passed as f64 / total as f64
    } else {
        0.0
    };
    let valid = total_failed == 0 && total > 0;

    Ok((valid, score, all_errors))
}

#[tauri::command]
pub async fn evaluate_scaffold_project(db: State<'_, Arc<DbState>>, session_id: String, path: String) -> Result<(bool, f64, Vec<String>), String> {
    // Query full exchange data for evaluation
    let exchanges: Vec<RecordedExchange> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut s = conn.prepare(
            "SELECT method, path, req_body, resp_status, resp_headers, resp_body \
             FROM http_requests WHERE session_id=?1 LIMIT 30"
        ).map_err(|e| e.to_string())?;
        let rows: Vec<RecordedExchange> = s.query_map(params![session_id], |row| {
            let method: String = row.get(0)?;
            let path: String = row.get(1)?;
            let req_body_opt: Option<Vec<u8>> = row.get(2)?;
            let resp_status: Option<u16> = row.get(3)?;
            let resp_headers_json: String = row.get(4)?;
            let resp_body_opt: Option<Vec<u8>> = row.get(5)?;
            let req_body = req_body_opt.map(|b| String::from_utf8_lossy(&b).to_string());
            let resp_headers: Vec<(String, String)> = serde_json::from_str(&resp_headers_json).unwrap_or_default();
            let resp_body = resp_body_opt.map(|b| String::from_utf8_lossy(&b).to_string());
            Ok(RecordedExchange {
                method,
                path,
                req_body,
                resp_status: resp_status.unwrap_or(200),
                resp_headers,
                resp_body,
            })
        }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
        rows
    };

    eval_scaffold(&path, exchanges).await
}
