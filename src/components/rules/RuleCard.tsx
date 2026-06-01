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
      className="flex items-center gap-3 p-2 px-3 bg-[#12121a] rounded-lg hover:bg-surface-tertiary transition-colors cursor-pointer"
      onClick={onEdit}
      style={{ gap: 12 }}
    >
      {/* Toggle switch - 36x22px with cornerRadius 4 */}
      <div
        onClick={(e) => {
          e.stopPropagation();
          onToggle(!isEnabled);
        }}
        className={`w-9 h-[22px] rounded flex items-center justify-center transition-all cursor-pointer ${
          isEnabled
            ? 'bg-[#ff4d4d40] border border-[#ff4d4d50]'
            : 'bg-[#22c55e40] border border-[#2a2a4a]'
        }`}
        style={{ borderWidth: 1 }}
      >
        <span className={`text-[11px] font-medium ${isEnabled ? 'text-[#ff6b6b]' : 'text-[#22c55e]'}`}>
          {isEnabled ? 'ON' : 'OFF'}
        </span>
      </div>

      {/* Domain and rule type */}
      <div className="flex-1 min-w-0">
        <div className="text-[#fff] font-mono text-[13px] truncate" style={{ fontFamily: 'JetBrains Mono, monospace' }}>{domain}</div>
        <div className="text-[#666688] text-[10px]">{ruleType} • {category}</div>
      </div>

      {/* Blocked count or paused */}
      <span className={isEnabled ? 'text-[#ff6b6b] text-[11px] shrink-0' : 'text-[#666688] text-[11px] shrink-0'}>
        {isEnabled ? `${blocked} blocked` : 'paused'}
      </span>

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
