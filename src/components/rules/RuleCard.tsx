import { Pencil, Trash2, Check, ArrowUp, ArrowDown } from "lucide-react";
import type { Rule, RuleAction } from "../../generated/desktop-contract";

interface RuleCardProps {
  rule: Rule;
  isFirst?: boolean;
  isLast?: boolean;
  onEdit: () => void;
  onDelete: () => void;
  onToggle: (enabled: boolean) => void;
  onMoveUp?: () => void;
  onMoveDown?: () => void;
}

export function RuleCard({ rule, isFirst, isLast, onEdit, onDelete, onToggle, onMoveUp, onMoveDown }: RuleCardProps) {
  const actionColors: Partial<Record<RuleAction["type"], string>> = {
    DIRECT: "badge-direct",
    PROXY: "badge-proxy",
    REJECT: "badge-reject",
    MAPREMOTE: "badge-info",
    MAPLOCAL: "badge-warning",
  };
  const actionLabel = "target" in rule.action
    ? `${rule.action.type}: ${rule.action.target}`
    : rule.action.type;

  return (
    <div className={`card hover:border-border-light transition-colors cursor-pointer ${!rule.enabled ? "opacity-50" : ""}`}>
      <div className="flex justify-between items-start mb-2">
        <div className="flex items-center gap-2">
          <button
            onClick={() => onToggle(!rule.enabled)}
            aria-label={rule.enabled ? "Disable rule" : "Enable rule"}
            className={`w-8 h-5 rounded-full flex items-center justify-center transition-colors ${
              rule.enabled ? "bg-accent-blue" : "bg-surface-tertiary"
            }`}
          >
            <Check size={14} className="text-white" />
          </button>
          <h3 className="font-semibold text-text-primary">{rule.name || "Unnamed Rule"}</h3>
        </div>
        <span className={`badge ${actionColors[rule.action.type] || "badge-unknown"}`}>
          {actionLabel}
        </span>
      </div>

      <div className="space-y-1 text-sm text-text-secondary mb-3">
        <p>
          <span className="font-medium">{rule.pattern}:</span> {rule.value}
        </p>
        {rule.comment && <p className="text-text-muted">{rule.comment}</p>}
      </div>

      <div className="flex justify-between items-center">
        <div className="flex gap-1">
          {onMoveUp && <button onClick={onMoveUp} disabled={isFirst} className="btn btn-ghost btn-sm" title="Move up" aria-label="Move rule up"><ArrowUp size={14} /></button>}
          {onMoveDown && <button onClick={onMoveDown} disabled={isLast} className="btn btn-ghost btn-sm" title="Move down" aria-label="Move rule down"><ArrowDown size={14} /></button>}
        </div>
        <div className="flex gap-2">
          <button onClick={onEdit} className="btn btn-ghost btn-sm" aria-label="Edit rule"><Pencil size={16} /></button>
          <button onClick={onDelete} className="btn btn-ghost btn-sm hover:!text-accent-red" aria-label="Delete rule"><Trash2 size={16} /></button>
        </div>
      </div>
    </div>
  );
}
