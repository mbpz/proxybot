import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Rule, RulePattern, RuleAction } from "../types";

export function RulesPage() {
  const [rules, setRules] = useState<Rule[]>([]);
  const [ruleFiles, setRuleFiles] = useState<string[]>([]);
  const [selectedRuleFile, setSelectedRuleFile] = useState("rules.yaml");
  const [showRuleEditor, setShowRuleEditor] = useState(false);
  const [editingRule, setEditingRule] = useState<Rule | null>(null);

  useEffect(() => {
    loadRuleFiles();
    loadRules();
  }, []);

  const loadRuleFiles = async () => {
    try {
      const files = await invoke<string[]>("list_rule_files");
      setRuleFiles(files.length > 0 ? files : ["rules.yaml"]);
      if (files.length > 0 && !files.includes(selectedRuleFile)) {
        setSelectedRuleFile(files[0]);
      }
    } catch (e) {
      console.error("Failed to load rule files:", e);
      setRuleFiles(["rules.yaml"]);
    }
  };

  const loadRules = async () => {
    try {
      setRules(await invoke<Rule[]>("get_rules"));
    } catch (e) {
      console.error("Failed to load rules:", e);
    }
  };

  const saveRule = async (rule: Rule) => {
    try {
      await invoke("save_rule", { rule, filename: selectedRuleFile });
      await loadRules();
      setShowRuleEditor(false);
      setEditingRule(null);
    } catch (e) {
      alert(String(e));
    }
  };

  const deleteRule = async (rule: Rule) => {
    try {
      await invoke("delete_rule", { rule, filename: selectedRuleFile });
      await loadRules();
    } catch (e) {
      alert(String(e));
    }
  };

  const moveRule = async (index: number, direction: "up" | "down") => {
    const newRules = [...rules];
    const targetIndex = direction === "up" ? index - 1 : index + 1;
    if (targetIndex < 0 || targetIndex >= newRules.length) return;
    [newRules[index], newRules[targetIndex]] = [newRules[targetIndex], newRules[index]];
    try {
      await invoke("reorder_rules", { rules: newRules, filename: selectedRuleFile });
      await loadRules();
    } catch (e) {
      alert(String(e));
    }
  };

  return (
    <div>
      <div className="panel" style={{ marginBottom: "var(--space-4)" }}>
        <div className="panel-header">
          <span className="panel-title">Routing Rules</span>
          <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
            <select value={selectedRuleFile} onChange={(e) => setSelectedRuleFile(e.target.value)} style={{ width: 140 }}>
              {ruleFiles.map((f) => <option key={f} value={f}>{f}</option>)}
            </select>
            <button className="btn btn-sm btn-secondary"
              onClick={() => { setEditingRule({ pattern: "DOMAIN-SUFFIX", value: "", action: "DIRECT" }); setShowRuleEditor(true); }}>
              + Add Rule
            </button>
          </div>
        </div>
        <div style={{ maxHeight: 400, overflowY: "auto" }}>
          {rules.length === 0 ? (
            <div className="empty-state">
              <div className="empty-state-icon">📋</div>
              <div className="empty-state-title">No rules defined</div>
              <div className="empty-state-description">Click "Add Rule" to create your first routing rule.</div>
            </div>
          ) : (
            <table className="table">
              <thead><tr><th>Pattern</th><th>Value</th><th>Action</th><th style={{ width: 120 }}>Controls</th></tr></thead>
              <tbody>
                {rules.map((rule, idx) => (
                  <tr key={`${rule.pattern}-${rule.value}-${idx}`}>
                    <td><span className={`badge ${rule.action === "DIRECT" ? "badge-direct" : rule.action === "PROXY" ? "badge-proxy" : "badge-reject"}`}>{rule.pattern}</span></td>
                    <td className="mono text-sm">{rule.value}</td>
                    <td><span className={`badge ${rule.action === "DIRECT" ? "badge-direct" : rule.action === "PROXY" ? "badge-proxy" : "badge-reject"}`}>{rule.action}</span></td>
                    <td>
                      <div style={{ display: "flex", gap: "var(--space-1)" }}>
                        <button className="btn btn-sm btn-ghost" onClick={() => moveRule(idx, "up")} disabled={idx === 0} title="Move up">↑</button>
                        <button className="btn btn-sm btn-ghost" onClick={() => moveRule(idx, "down")} disabled={idx === rules.length - 1} title="Move down">↓</button>
                        <button className="btn btn-sm btn-ghost" onClick={() => { setEditingRule(rule); setShowRuleEditor(true); }}>Edit</button>
                        <button className="btn btn-sm btn-ghost" onClick={() => deleteRule(rule)}>×</button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>

      {showRuleEditor && editingRule && (
        <div style={{ position: "fixed", top: 0, left: 0, right: 0, bottom: 0, background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
          <div className="panel" style={{ width: 480 }}>
            <div className="panel-header">
              <span className="panel-title">Add / Edit Rule</span>
              <button className="btn btn-sm btn-ghost" onClick={() => { setShowRuleEditor(false); setEditingRule(null); }}>×</button>
            </div>
            <div className="panel-body">
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
                <div>
                  <label className="text-sm text-muted" style={{ display: "block", marginBottom: 4 }}>Pattern</label>
                  <select value={editingRule.pattern} onChange={(e) => setEditingRule({ ...editingRule, pattern: e.target.value as RulePattern })} style={{ width: "100%" }}>
                    <option value="DOMAIN">DOMAIN (exact match)</option>
                    <option value="DOMAIN-SUFFIX">DOMAIN-SUFFIX (matches subdomains)</option>
                    <option value="DOMAIN-KEYWORD">DOMAIN-KEYWORD (contains)</option>
                    <option value="IP-CIDR">IP-CIDR (e.g., 10.0.0.0/8)</option>
                  </select>
                </div>
                <div>
                  <label className="text-sm text-muted" style={{ display: "block", marginBottom: 4 }}>Value</label>
                  <input type="text" value={editingRule.value} onChange={(e) => setEditingRule({ ...editingRule, value: e.target.value })}
                    placeholder={editingRule.pattern === "IP-CIDR" ? "10.0.0.0/8" : "example.com"} style={{ width: "100%" }} />
                </div>
                <div>
                  <label className="text-sm text-muted" style={{ display: "block", marginBottom: 4 }}>Action</label>
                  <select value={editingRule.action} onChange={(e) => setEditingRule({ ...editingRule, action: e.target.value as RuleAction })} style={{ width: "100%" }}>
                    <option value="DIRECT">DIRECT (bypass proxy)</option>
                    <option value="PROXY">PROXY (send through proxy)</option>
                    <option value="REJECT">REJECT (block connection)</option>
                  </select>
                </div>
                <div style={{ display: "flex", gap: "var(--space-2)", justifyContent: "flex-end" }}>
                  <button className="btn btn-sm btn-secondary" onClick={() => { setShowRuleEditor(false); setEditingRule(null); }}>Cancel</button>
                  <button className="btn btn-sm btn-primary" onClick={() => saveRule(editingRule)}>Save</button>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
