import { Pencil, Trash2, Check } from "lucide-react";

interface Rule {
  pattern: string;
  value: string;
  action: string;
  name: string;
  priority: number;
  enabled: boolean;
  comment: string;
}

interface RuleCardProps {
  rule: Rule;
  onEdit: () => void;
  onDelete: () => void;
  onToggle: (enabled: boolean) => void;
}

export function RuleCard({ rule, onEdit, onDelete, onToggle }: RuleCardProps) {
  const actionColors: Record<string, string> = {
    DIRECT: "badge-direct",
    PROXY: "badge-proxy",
    REJECT: "badge-reject",
    MAPREMOTE: "badge-info",
    MAPLOCAL: "badge-warning",
  };

  return (
    <div className={`card hover:border-border-light transition-colors cursor-pointer ${!rule.enabled ? "opacity-50" : ""}`}>
      <div className="flex justify-between items-start mb-2">
        <div className="flex items-center gap-2">
          <button
            onClick={() => onToggle(!rule.enabled)}
            className={`w-8 h-5 rounded-full flex items-center justify-center transition-colors ${
              rule.enabled ? "bg-accent-blue" : "bg-surface-tertiary"
            }`}
          >
            <Check size={14} className="text-white" />
          </button>
          <h3 className="font-semibold text-text-primary">{rule.name || "Unnamed Rule"}</h3>
        </div>
        <span className={`badge ${actionColors[rule.action] || "badge-unknown"}`}>
          {rule.action}
        </span>
      </div>

      <div className="space-y-1 text-sm text-text-secondary mb-3">
        <p>
          <span className="font-medium">{rule.pattern}:</span> {rule.value}
        </p>
        {rule.comment && <p className="text-text-muted">{rule.comment}</p>}
      </div>

      <div className="flex justify-end gap-2">
        <button
          onClick={onEdit}
          className="btn btn-ghost btn-sm"
        >
          <Pencil size={16} />
        </button>
        <button
          onClick={onDelete}
          className="btn btn-ghost btn-sm hover:!text-accent-red"
        >
          <Trash2 size={16} />
        </button>
      </div>
    </div>
  );
}
