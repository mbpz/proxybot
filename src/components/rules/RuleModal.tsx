import { useState } from "react";
import type {
  BreakpointTarget,
  Rule,
  RuleAction,
  RulePattern,
} from "../../generated/desktop-contract";

interface RuleModalProps {
  rule: Rule | null;
  onSave: (rule: Rule) => void | Promise<void>;
  onClose: () => void;
}

const patterns: RulePattern[] = [
  "DOMAIN",
  "DOMAIN-SUFFIX",
  "DOMAIN-KEYWORD",
  "IP-CIDR",
  "GEOIP",
  "RULE-SET",
];
const actions: Array<RuleAction["type"]> = [
  "DIRECT",
  "PROXY",
  "REJECT",
  "MAPREMOTE",
  "MAPLOCAL",
  "BREAKPOINT",
];

function createAction(type: RuleAction["type"], target = ""): RuleAction {
  switch (type) {
    case "DIRECT":
      return { type: "DIRECT" };
    case "PROXY":
      return { type: "PROXY" };
    case "REJECT":
      return { type: "REJECT" };
    case "MAPREMOTE":
      return { type: "MAPREMOTE", target };
    case "MAPLOCAL":
      return { type: "MAPLOCAL", target };
    case "BREAKPOINT":
      return {
        type: "BREAKPOINT",
        target:
          target === "REQUEST" || target === "RESPONSE" || target === "BOTH"
            ? target
            : "BOTH",
      };
  }
}

export function RuleModal({ rule, onSave, onClose }: RuleModalProps) {
  const [formData, setFormData] = useState<Rule>({
    pattern: rule?.pattern ?? "DOMAIN-SUFFIX",
    value: rule?.value ?? "",
    action: rule?.action ?? { type: "DIRECT" },
    name: rule?.name ?? "",
    priority: rule?.priority ?? 100,
    enabled: rule?.enabled ?? true,
    comment: rule?.comment ?? "",
  });
  const actionTarget = "target" in formData.action ? formData.action.target : "";

  function handleSubmit(event: React.FormEvent) {
    event.preventDefault();
    void onSave(formData);
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="card w-full max-w-md p-6 shadow-lg">
        <h2 className="text-xl font-bold mb-4">{rule ? "Edit Rule" : "Add Rule"}</h2>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Name</label>
            <input
              type="text"
              value={formData.name}
              onChange={(event) => setFormData({ ...formData, name: event.target.value })}
              className="w-full px-3 py-2"
              placeholder="My Rule"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Pattern</label>
            <select
              value={formData.pattern}
              onChange={(event) => setFormData({ ...formData, pattern: event.target.value as RulePattern })}
              className="w-full px-3 py-2"
            >
              {patterns.map((pattern) => (
                <option key={pattern} value={pattern}>{pattern}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Value</label>
            <input
              type="text"
              value={formData.value}
              onChange={(event) => setFormData({ ...formData, value: event.target.value })}
              className="w-full px-3 py-2"
              placeholder="example.com"
              required
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Action</label>
            <select
              value={formData.action.type}
              onChange={(event) => {
                const type = event.target.value as RuleAction["type"];
                setFormData({ ...formData, action: createAction(type, actionTarget) });
              }}
              className="w-full px-3 py-2"
            >
              {actions.map((action) => (
                <option key={action} value={action}>{action}</option>
              ))}
            </select>
          </div>

          {(formData.action.type === "MAPREMOTE" || formData.action.type === "MAPLOCAL") && (
            <div>
              <label className="block text-sm font-medium text-text-secondary mb-1">Target</label>
              <input
                type="text"
                value={actionTarget}
                onChange={(event) => setFormData({
                  ...formData,
                  action: createAction(formData.action.type, event.target.value),
                })}
                className="w-full px-3 py-2"
                placeholder={formData.action.type === "MAPREMOTE" ? "https://mock.example" : "/path/to/file"}
                required
              />
            </div>
          )}

          {formData.action.type === "BREAKPOINT" && (
            <div>
              <label className="block text-sm font-medium text-text-secondary mb-1">Breakpoint target</label>
              <select
                value={actionTarget}
                onChange={(event) => setFormData({
                  ...formData,
                  action: { type: "BREAKPOINT", target: event.target.value as BreakpointTarget },
                })}
                className="w-full px-3 py-2"
              >
                <option value="REQUEST">REQUEST</option>
                <option value="RESPONSE">RESPONSE</option>
                <option value="BOTH">BOTH</option>
              </select>
            </div>
          )}

          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Priority</label>
            <input
              type="number"
              value={formData.priority}
              onChange={(event) => setFormData({
                ...formData,
                priority: Number.parseInt(event.target.value, 10) || 100,
              })}
              className="w-full px-3 py-2"
              min="1"
              max="255"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Comment</label>
            <input
              type="text"
              value={formData.comment}
              onChange={(event) => setFormData({ ...formData, comment: event.target.value })}
              className="w-full px-3 py-2"
              placeholder="Optional comment"
            />
          </div>

          <div className="flex justify-end gap-3 pt-4">
            <button type="button" onClick={onClose} className="btn btn-ghost">
              Cancel
            </button>
            <button type="submit" className="btn btn-primary">
              Save
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
