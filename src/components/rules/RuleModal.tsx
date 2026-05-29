import { useState } from "react";

interface Rule {
  pattern: string;
  value: string;
  action: string;
  name: string;
  priority: number;
  enabled: boolean;
  comment: string;
}

interface RuleModalProps {
  rule: Rule | null;
  onSave: (rule: Rule) => void;
  onClose: () => void;
}

const patterns = ["DOMAIN", "DOMAIN-SUFFIX", "DOMAIN-KEYWORD", "IP-CIDR", "GEOIP", "RULE-SET"];
const actions = ["DIRECT", "PROXY", "REJECT", "MAPREMOTE", "MAPLOCAL", "BREAKPOINT"];

export function RuleModal({ rule, onSave, onClose }: RuleModalProps) {
  const [formData, setFormData] = useState<Rule>({
    pattern: rule?.pattern || "DOMAIN-SUFFIX",
    value: rule?.value || "",
    action: rule?.action || "DIRECT",
    name: rule?.name || "",
    priority: rule?.priority || 100,
    enabled: rule?.enabled ?? true,
    comment: rule?.comment || "",
  });

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    onSave(formData);
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
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              className="w-full px-3 py-2"
              placeholder="My Rule"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Pattern</label>
            <select
              value={formData.pattern}
              onChange={(e) => setFormData({ ...formData, pattern: e.target.value })}
              className="w-full px-3 py-2"
            >
              {patterns.map((p) => (
                <option key={p} value={p}>{p}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Value</label>
            <input
              type="text"
              value={formData.value}
              onChange={(e) => setFormData({ ...formData, value: e.target.value })}
              className="w-full px-3 py-2"
              placeholder="example.com"
              required
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Action</label>
            <select
              value={formData.action}
              onChange={(e) => setFormData({ ...formData, action: e.target.value })}
              className="w-full px-3 py-2"
            >
              {actions.map((a) => (
                <option key={a} value={a}>{a}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1">Priority</label>
            <input
              type="number"
              value={formData.priority}
              onChange={(e) => setFormData({ ...formData, priority: parseInt(e.target.value) || 100 })}
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
              onChange={(e) => setFormData({ ...formData, comment: e.target.value })}
              className="w-full px-3 py-2"
              placeholder="Optional comment"
            />
          </div>

          <div className="flex justify-end gap-3 pt-4">
            <button
              type="button"
              onClick={onClose}
              className="btn btn-ghost"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary"
            >
              Save
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
