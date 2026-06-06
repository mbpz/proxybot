import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RuleCard } from "./RuleCard";
import { RuleModal } from "./RuleModal";
import { Button } from "../ui/Button";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonCard } from "../ui/skeleton";
import { safeInvoke, safeInvokeOr } from "../../utils/safeInvoke";
import { Shield, Search } from "lucide-react";

interface Rule {
  pattern: string;
  value: string;
  action: string;
  name: string;
  priority: number;
  enabled: boolean;
  comment: string;
}

export function RulesPage() {
  const [rules, setRules] = useState<Rule[]>([]);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<Rule | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [ruleFiles, setRuleFiles] = useState<string[]>([]);
  const [selectedFile, setSelectedFile] = useState("custom.yaml");
  const [testHost, setTestHost] = useState("");
  const [matchResult, setMatchResult] = useState<string | null>(null);

  useEffect(() => {
    loadRules();
    loadRuleFiles();
  }, []);

  async function loadRuleFiles() {
    try {
      const files = await invoke<string[] | null>("list_rule_files");
      if (Array.isArray(files)) {
        setRuleFiles(files);
      }
    } catch { /* ignore */ }
  }

  async function handleReorder(rule: Rule, direction: "up" | "down") {
    const idx = rules.indexOf(rule);
    if (idx < 0) return;
    const newIdx = direction === "up" ? idx - 1 : idx + 1;
    if (newIdx < 0 || newIdx >= rules.length) return;
    try {
      await invoke("reorder_rules", { from_index: idx, to_index: newIdx, filename: selectedFile });
      loadRules();
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleMatchHost() {
    if (!testHost) return;
    try {
      const result = await invoke<string | null>("match_host", { host: testHost });
      setMatchResult(result || "No match");
    } catch (err) {
      setMatchResult("Error: " + String(err));
    }
  }

  async function loadRules() {
    try {
      setLoading(true);
      setError(null);
      const result = await safeInvokeOr<Rule[]>("get_rules", []);
      setRules(result);
    } catch (err) {
      console.error("Failed to load rules:", err);
    } finally {
      setLoading(false);
    }
  }

  function handleAddRule() {
    setEditingRule(null);
    setModalOpen(true);
  }

  function handleEditRule(rule: Rule) {
    setEditingRule(rule);
    setModalOpen(true);
  }

  async function handleSaveRule(rule: Rule) {
    await safeInvoke("save_rule", { rule, filename: "custom.yaml" });
    setModalOpen(false);
    loadRules();
  }

  async function handleDeleteRule(rule: Rule) {
    await safeInvoke("delete_rule", { rule, filename: "custom.yaml" });
    loadRules();
  }

  async function handleToggleRule(rule: Rule, enabled: boolean) {
    await safeInvoke("save_rule", {
      rule: { ...rule, enabled },
      filename: "custom.yaml",
    });
    loadRules();
  }

  return (
    <div>
      <div className="panel">
        {/* Header */}
        <div className="panel-header">
          <div className="flex items-center gap-3">
            <span className="panel-title">Rules</span>
            <span className="text-sm text-muted">{rules.length} rules</span>
            {ruleFiles.length > 0 && (
              <select value={selectedFile} onChange={async (e) => { setSelectedFile(e.target.value); await loadRules(); }} style={{ width: 160 }}>
                {ruleFiles.map(f => <option key={f} value={f}>{f}</option>)}
              </select>
            )}
          </div>
          <div className="flex items-center gap-2">
            <input type="text" value={testHost} onChange={e => setTestHost(e.target.value)} placeholder="Test host..." style={{ width: 160 }} onKeyDown={e => e.key === "Enter" && handleMatchHost()} />
            <Button variant="secondary" size="sm" onClick={handleMatchHost}><Search size={14} /></Button>
            {matchResult && <span className="text-xs mono" style={{ color: matchResult === "No match" ? "var(--text-muted)" : "var(--accent-green)" }}>{matchResult}</span>}
            <Button variant="primary" size="sm" onClick={handleAddRule}>+ Add Rule</Button>
          </div>
        </div>

        {/* Error banner */}
        {error && (
          <div className="error-banner mx-4 mt-2">
            <span className="error-banner-message">{error}</span>
            <Button variant="secondary" size="sm" onClick={loadRules}>
              Retry
            </Button>
          </div>
        )}

        {/* Content */}
        <div className="panel-body">
          <ErrorBoundary>
            {loading ? (
              <div
                className="grid gap-4"
                style={{
                  gridTemplateColumns:
                    "repeat(auto-fill, minmax(300px, 1fr))",
                }}
              >
                <SkeletonCard />
                <SkeletonCard />
                <SkeletonCard />
              </div>
            ) : rules.length === 0 ? (
              <div className="empty-state">
                <Shield size={48} className="empty-state-icon" />
                <div className="empty-state-title">No rules configured</div>
                <div className="empty-state-description">
                  Click "Add Rule" to create your first routing rule.
                </div>
              </div>
            ) : (
              <div
                className="grid gap-4"
                style={{
                  gridTemplateColumns:
                    "repeat(auto-fill, minmax(300px, 1fr))",
                }}
              >
                {rules.map((rule, i) => (
                  <RuleCard
                    key={`${rule.pattern}-${rule.value}`}
                    rule={rule}
                    isFirst={i === 0}
                    isLast={i === rules.length - 1}
                    onEdit={() => handleEditRule(rule)}
                    onDelete={() => handleDeleteRule(rule)}
                    onToggle={(enabled) => handleToggleRule(rule, enabled)}
                    onMoveUp={() => handleReorder(rule, "up")}
                    onMoveDown={() => handleReorder(rule, "down")}
                  />
                ))}
              </div>
            )}
          </ErrorBoundary>
        </div>
      </div>

      {/* Rule editor modal */}
      {modalOpen && (
        <RuleModal
          rule={editingRule}
          onSave={handleSaveRule}
          onClose={() => setModalOpen(false)}
        />
      )}
    </div>
  );
}