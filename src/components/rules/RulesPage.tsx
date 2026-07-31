import { useEffect, useState } from "react";
import { Shield, Search } from "lucide-react";
import { desktop } from "../../desktop/contract";
import type { Rule, RuleAction } from "../../generated/desktop-contract";
import { RuleCard } from "./RuleCard";
import { RuleModal } from "./RuleModal";
import { Button } from "../ui/Button";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonCard } from "../ui/skeleton";

function actionLabel(action: RuleAction | null): string {
  if (!action) return "No match";
  return "target" in action ? `${action.type}: ${action.target}` : action.type;
}

export function RulesPage() {
  const [rules, setRules] = useState<Rule[]>([]);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<Rule | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [ruleFiles, setRuleFiles] = useState<string[]>(["custom.yaml"]);
  const [selectedFile, setSelectedFile] = useState("custom.yaml");
  const [testHost, setTestHost] = useState("");
  const [matchResult, setMatchResult] = useState<string | null>(null);

  useEffect(() => {
    void loadRuleFiles();
  }, []);

  useEffect(() => {
    void loadRules(selectedFile);
  }, [selectedFile]);

  async function loadRuleFiles() {
    try {
      const files = await desktop.call("list_rule_files", {});
      setRuleFiles(Array.from(new Set(["custom.yaml", ...files])).sort());
    } catch (cause) {
      setError(String(cause));
    }
  }

  async function loadRules(filename: string) {
    try {
      setLoading(true);
      setError(null);
      setRules(await desktop.call("get_rules", { filename }));
    } catch (cause) {
      setError(String(cause));
    } finally {
      setLoading(false);
    }
  }

  async function handleReorder(rule: Rule, direction: "up" | "down") {
    const fromIndex = rules.indexOf(rule);
    if (fromIndex < 0) return;
    const toIndex = direction === "up" ? fromIndex - 1 : fromIndex + 1;
    if (toIndex < 0 || toIndex >= rules.length) return;

    try {
      setError(null);
      await desktop.call("reorder_rules", { fromIndex, toIndex, filename: selectedFile });
      await loadRules(selectedFile);
    } catch (cause) {
      setError(String(cause));
    }
  }

  async function handleMatchHost() {
    if (!testHost) return;
    try {
      const result = await desktop.call("match_host", { host: testHost, ip: null });
      setMatchResult(actionLabel(result));
    } catch (cause) {
      setMatchResult(`Error: ${String(cause)}`);
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
    try {
      setError(null);
      await desktop.call("save_rule", {
        rule,
        filename: selectedFile,
        originalRule: editingRule,
      });
      setModalOpen(false);
      setEditingRule(null);
      await loadRules(selectedFile);
      await loadRuleFiles();
    } catch (cause) {
      setError(String(cause));
    }
  }

  async function handleDeleteRule(rule: Rule) {
    try {
      setError(null);
      await desktop.call("delete_rule", { rule, filename: selectedFile });
      await loadRules(selectedFile);
    } catch (cause) {
      setError(String(cause));
    }
  }

  async function handleToggleRule(rule: Rule, enabled: boolean) {
    try {
      setError(null);
      await desktop.call("save_rule", {
        rule: { ...rule, enabled },
        filename: selectedFile,
        originalRule: rule,
      });
      await loadRules(selectedFile);
    } catch (cause) {
      setError(String(cause));
    }
  }

  return (
    <div>
      <div className="panel">
        <div className="panel-header">
          <div className="flex items-center gap-3">
            <span className="panel-title">Rules</span>
            <span className="text-sm text-muted">{rules.length} rules</span>
            <select
              value={selectedFile}
              onChange={(event) => setSelectedFile(event.target.value)}
              style={{ width: 160 }}
            >
              {ruleFiles.map((filename) => (
                <option key={filename} value={filename}>{filename}</option>
              ))}
            </select>
          </div>
          <div className="flex items-center gap-2">
            <input
              type="text"
              value={testHost}
              onChange={(event) => setTestHost(event.target.value)}
              placeholder="Test host..."
              style={{ width: 160 }}
              onKeyDown={(event) => event.key === "Enter" && void handleMatchHost()}
            />
            <Button variant="secondary" size="sm" onClick={handleMatchHost}><Search size={14} /></Button>
            {matchResult && (
              <span
                className="text-xs mono"
                style={{ color: matchResult === "No match" ? "var(--text-muted)" : "var(--accent-green)" }}
              >
                {matchResult}
              </span>
            )}
            <Button variant="primary" size="sm" onClick={handleAddRule}>+ Add Rule</Button>
          </div>
        </div>

        {error && (
          <div className="error-banner mx-4 mt-2">
            <span className="error-banner-message">{error}</span>
            <Button variant="secondary" size="sm" onClick={() => void loadRules(selectedFile)}>
              Retry
            </Button>
          </div>
        )}

        <div className="panel-body">
          <ErrorBoundary>
            {loading ? (
              <div
                className="grid gap-4"
                style={{ gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))" }}
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
                style={{ gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))" }}
              >
                {rules.map((rule, index) => (
                  <RuleCard
                    key={`${rule.pattern}-${rule.value}-${rule.priority}-${index}`}
                    rule={rule}
                    isFirst={index === 0}
                    isLast={index === rules.length - 1}
                    onEdit={() => handleEditRule(rule)}
                    onDelete={() => void handleDeleteRule(rule)}
                    onToggle={(enabled) => void handleToggleRule(rule, enabled)}
                    onMoveUp={() => void handleReorder(rule, "up")}
                    onMoveDown={() => void handleReorder(rule, "down")}
                  />
                ))}
              </div>
            )}
          </ErrorBoundary>
        </div>
      </div>

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
