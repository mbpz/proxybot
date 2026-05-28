import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RuleCard } from "./RuleCard";
import { RuleModal } from "./RuleModal";
import { Button } from "../ui/Button";
import { ErrorBoundary } from "../ui/error-boundary";
import { SkeletonCard } from "../ui/skeleton";
import { safeInvokeOr } from "../../utils/safeInvoke";

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

  useEffect(() => {
    loadRules();
  }, []);

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
    try {
      await invoke("save_rule", { rule, filename: "custom.yaml" });
      setModalOpen(false);
      loadRules();
    } catch (err) {
      console.error("Failed to save rule:", err);
    }
  }

  async function handleDeleteRule(rule: Rule) {
    try {
      await invoke("delete_rule", { rule, filename: "custom.yaml" });
      loadRules();
    } catch (err) {
      console.error("Failed to delete rule:", err);
    }
  }

  async function handleToggleRule(rule: Rule, enabled: boolean) {
    try {
      await invoke("save_rule", {
        rule: { ...rule, enabled },
        filename: "custom.yaml",
      });
      loadRules();
    } catch (err) {
      console.error("Failed to toggle rule:", err);
    }
  }

  return (
    <div>
      <div className="panel">
        {/* Header */}
        <div className="panel-header">
          <div className="flex items-center gap-3">
            <span className="panel-title">Rules</span>
            <span className="text-sm text-muted">{rules.length} rules</span>
          </div>
          <Button variant="primary" size="sm" onClick={handleAddRule}>
            + Add Rule
          </Button>
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
                <div className="empty-state-icon">📋</div>
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
                {rules.map((rule) => (
                  <RuleCard
                    key={`${rule.pattern}-${rule.value}`}
                    rule={rule}
                    onEdit={() => handleEditRule(rule)}
                    onDelete={() => handleDeleteRule(rule)}
                    onToggle={(enabled) => handleToggleRule(rule, enabled)}
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