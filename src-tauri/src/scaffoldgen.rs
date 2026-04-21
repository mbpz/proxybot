//! React scaffold generator for ProxyBot.

use crate::db::DbState;
use crate::infer::{ApiInterface, InferredApi};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaffoldComponent {
    pub name: String,
    pub route_path: String,
    pub file_path: String,
    pub content: String,
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

fn get_anthropic_api_key() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("CLAUDE_API_KEY"))
        .ok()
}

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

async fn eval_scaffold(path: &str, reqs: &[(String, String, String)]) -> Result<(bool, f64, Vec<String>), String> {
    let key = get_anthropic_api_key().ok_or("ANTHROPIC_API_KEY not set")?;
    let mut p = String::from("Eval scaffold vs real traffic.\nPath: ");
    p.push_str(path);
    p.push_str("\nRequests:\n");
    for (i, (m, pa, b)) in reqs.iter().take(20).enumerate() {
        p.push_str(&format!("{}. {} {} - {}\n", i + 1, m, pa, b));
    }
    p.push_str("\nOutput JSON: {\"valid\":true|false,\"score\":0.0-1.0,\"errors\":[]}");
    let out = call_claude_api(&p, &key).await?;
    #[derive(Deserialize)]
    struct R { valid: bool, score: f64, errors: Vec<String> }
    let r: R = serde_json::from_str(&out).map_err(|e| format!("Parse err: {} / {}", e, out))?;
    Ok((r.valid, r.score, r.errors))
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
            comps.push(ScaffoldComponent { name: cn.clone(), route_path: infer_route(&a.path), file_path: pf, content: pc });
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

#[tauri::command]
pub async fn evaluate_scaffold_project(db: State<'_, Arc<DbState>>, session_id: String, path: String) -> Result<(bool, f64, Vec<String>), String> {
    let reqs = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut s = conn.prepare("SELECT method,path,resp_body FROM http_requests WHERE session_id=?1 LIMIT 20").map_err(|e| e.to_string())?;
        let rows: Vec<(String, String, String)> = s.query_map(params![session_id], |row| {
            let m: String = row.get(0)?;
            let p: String = row.get(1)?;
            let b: Option<Vec<u8>> = row.get(2)?;
            Ok((m, p, b.as_ref().map(|v| String::from_utf8_lossy(v).to_string()).unwrap_or_default()))
        }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
        rows
    };
    eval_scaffold(&path, &reqs).await
}
