import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RuleCard } from "./RuleCard";
import { RuleModal } from "./RuleModal";

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
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadRules();
  }, []);

  async function loadRules() {
    try {
      setError(null);
      const result = await invoke<Rule[]>("get_rules");
      setRules(result);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      console.error("Failed to load rules:", err);
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
      await invoke("save_rule", { rule: { ...rule, enabled }, filename: "custom.yaml" });
      loadRules();
    } catch (err) {
      console.error("Failed to toggle rule:", err);
    }
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Rules</h1>
        <button
          onClick={handleAddRule}
          className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
        >
          Add Rule
        </button>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-100 text-red-700 rounded">{error}</div>
      )}

      <div className="grid gap-4 grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
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

      {rules.length === 0 && (
        <p className="text-gray-500 text-center py-8">No rules configured yet.</p>
      )}

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