import { useState } from "react";
import type {
  BreakpointTarget,
  Rule,
  RuleAction,
  RulePattern,
} from "../../generated/desktop-contract";
import { Dialog } from "../ui/Dialog";
import { InlineNotice } from "../ui/states";

interface RuleModalProps {
  rule: Rule | null;
  error?: string | null;
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

export function RuleModal({ rule, error, onSave, onClose }: RuleModalProps) {
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
    <Dialog open title={rule ? "Edit Rule" : "Add Rule"} onClose={onClose}>
      <form onSubmit={handleSubmit} className="space-y-4">
        {error && (
          <InlineNotice variant="error">
            Could not save rule: {error}
          </InlineNotice>
        )}

        <div>
          <label htmlFor="rule-name" className="block text-sm font-medium text-text-secondary mb-1">Name</label>
          <input
            id="rule-name"
            type="text"
            value={formData.name}
            onChange={(event) => setFormData({ ...formData, name: event.target.value })}
            className="w-full px-3 py-2"
            placeholder="My Rule"
          />
        </div>

        <div>
          <label htmlFor="rule-pattern" className="block text-sm font-medium text-text-secondary mb-1">Pattern</label>
          <select
            id="rule-pattern"
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
          <label htmlFor="rule-value" className="block text-sm font-medium text-text-secondary mb-1">Value</label>
          <input
            id="rule-value"
            type="text"
            value={formData.value}
            onChange={(event) => setFormData({ ...formData, value: event.target.value })}
            className="w-full px-3 py-2"
            placeholder="example.com"
            required
          />
        </div>

        <div>
          <label htmlFor="rule-action" className="block text-sm font-medium text-text-secondary mb-1">Action</label>
          <select
            id="rule-action"
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
            <label htmlFor="rule-target" className="block text-sm font-medium text-text-secondary mb-1">Target</label>
            <input
              id="rule-target"
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
            <label htmlFor="rule-breakpoint" className="block text-sm font-medium text-text-secondary mb-1">Breakpoint target</label>
            <select
              id="rule-breakpoint"
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
          <label htmlFor="rule-priority" className="block text-sm font-medium text-text-secondary mb-1">Priority</label>
          <input
            id="rule-priority"
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
          <label htmlFor="rule-comment" className="block text-sm font-medium text-text-secondary mb-1">Comment</label>
          <input
            id="rule-comment"
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
    </Dialog>
  );
}
