import { Pencil, Trash2 } from "lucide-react";

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
  const isEnabled = rule.enabled;
  const domain = `${rule.pattern}:${rule.value}`;
  const ruleType = rule.action;
  const category = rule.comment || "general";
  const blocked = 0; // Placeholder - actual implementation would track this

  return (
    <div
      className="flex items-center gap-3 p-3 bg-surface-secondary rounded-lg hover:bg-surface-tertiary transition-colors cursor-pointer"
      onClick={onEdit}
    >
      {/* Toggle switch */}
      <div
        onClick={(e) => {
          e.stopPropagation();
          onToggle(!isEnabled);
        }}
        className={`w-9 h-5.5 rounded-full flex items-center justify-center transition-all cursor-pointer ${
          isEnabled
            ? 'bg-accent-red/40 border border-accent-red/50'
            : 'bg-surface-tertiary border border-border'
        }`}
      >
        <span className={`text-xs font-medium ${isEnabled ? 'text-accent-red' : 'text-text-muted'}`}>
          {isEnabled ? 'ON' : 'OFF'}
        </span>
      </div>

      {/* Domain and rule type */}
      <div className="flex-1 min-w-0">
        <div className="text-text-primary font-mono text-sm truncate">{domain}</div>
        <div className="text-text-muted text-xs">{ruleType} • {category}</div>
      </div>

      {/* Blocked count */}
      <span className="text-accent-red text-xs shrink-0">{blocked} blocked</span>

      {/* Action buttons */}
      <div className="flex items-center gap-1 shrink-0" onClick={(e) => e.stopPropagation()}>
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
