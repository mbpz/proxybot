import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RuleCard } from "./RuleCard";
import { RuleModal } from "./RuleModal";

type RulePattern = "DOMAIN" | "DOMAIN-SUFFIX" | "DOMAIN-KEYWORD" | "IP-CIDR" | "GEOIP" | "RULE-SET";
type RuleAction = "DIRECT" | "PROXY" | "REJECT";

interface Rule {
  pattern: RulePattern;
  value: string;
  action: RuleAction;
  name: string;
  priority: number;
  enabled: boolean;
  comment: string;
}

export function RulesPage() {
  const [rules, setRules] = useState<Rule[]>([]);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<Rule | null>(null);

  useEffect(() => {
    loadRules();
  }, []);

  async function loadRules() {
    try {
      const result = await invoke<Rule[]>("get_rules");
      setRules(result);
    } catch (err) {
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

      <div className="grid gap-4 grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
        {rules.map((rule, index) => (
          <RuleCard
            key={index}
            rule={rule}
            onEdit={() => handleEditRule(rule)}
            onDelete={() => handleDeleteRule(rule)}
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