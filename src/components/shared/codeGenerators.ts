export function generateCurl(method: string, url: string, headers: Record<string, string>, body: string): string {
  const parts: string[] = ["curl"];
  if (method !== "GET" && method !== "POST") {
    parts.push(`-X ${method}`);
  } else if (method === "GET" && body) {
    parts.push("-X GET");
  }
  for (const [k, v] of Object.entries(headers)) {
    if (!k.toLowerCase().includes("host")) {
      parts.push(`-H '${escapeShell(v)}'`);
    }
  }
  if (body) {
    parts.push(`-d '${escapeShell(body)}'`);
  }
  parts.push(`'${url}'`);
  return parts.join(" \\\n  ");
}

function escapeShell(s: string): string {
  return s.replace(/'/g, "'\\''");
}

export function generateFetch(method: string, url: string, headers: Record<string, string>, body: string): string {
  const lines: string[] = [`fetch('${url}', {`];
  lines.push(`  method: '${method}',`);
  const headerKeys = Object.keys(headers).filter(k => !k.toLowerCase().includes("host"));
  if (headerKeys.length > 0) {
    lines.push("  headers: {");
    for (const k of headerKeys) {
      lines.push(`    '${k}': '${headers[k].replace(/'/g, "\\'")}',`);
    }
    lines.push("  },");
  }
  if (body) {
    lines.push(`  body: ${JSON.stringify(body)},`);
  }
  lines.push("})");
  lines.push("  .then(res => res.json())");
  lines.push("  .then(data => console.log(data));");
  return lines.join("\n");
}

export function generatePython(method: string, url: string, headers: Record<string, string>, body: string): string {
  const lines: string[] = ["import requests", ""];
  lines.push("response = requests.request(");
  lines.push(`    method='${method}',`);
  lines.push(`    url='${url}',`);
  const headerKeys = Object.keys(headers).filter(k => !k.toLowerCase().includes("host"));
  if (headerKeys.length > 0) {
    lines.push("    headers={");
    for (const k of headerKeys) {
      lines.push(`        '${k}': '${headers[k].replace(/'/g, "\\'")}',`);
    }
    lines.push("    },");
  }
  if (body) {
    lines.push(`    data='${body.replace(/'/g, "\\'")}',`);
  }
  lines.push(")");
  lines.push("");
  lines.push("print(response.status_code)");
  lines.push("print(response.json())");
  return lines.join("\n");
}

export function generateGo(method: string, url: string, headers: Record<string, string>, body: string): string {
  const lines: string[] = [
    "package main", "",
    "import (",
    '    "fmt"',
    '    "io"',
    '    "net/http"',
    '    "strings"',
    ")", "",
    "func main() {",
  ];
  if (body) {
    const escaped = body.replace(/\\/g, "\\\\").replace(/`/g, "\\`");
    lines.push(`    body := strings.NewReader(\`${escaped}\`)`);
    lines.push(`    req, _ := http.NewRequest("${method}", "${url}", body)`);
  } else {
    lines.push(`    req, _ := http.NewRequest("${method}", "${url}", nil)`);
  }
  for (const [k, v] of Object.entries(headers)) {
    if (!k.toLowerCase().includes("host")) {
      lines.push(`    req.Header.Set("${k}", "${v.replace(/"/g, '\\"')}")`);
    }
  }
  lines.push("");
  lines.push('    client := &http.Client{}');
  lines.push("    resp, _ := client.Do(req)");
  lines.push("    defer resp.Body.Close()");
  lines.push("");
  lines.push("    data, _ := io.ReadAll(resp.Body)");
  lines.push("    fmt.Println(string(data))");
  lines.push("}");
  return lines.join("\n");
}
