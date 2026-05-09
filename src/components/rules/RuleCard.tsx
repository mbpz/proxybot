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
    DIRECT: "bg-green-100 text-green-800",
    PROXY: "bg-blue-100 text-blue-800",
    REJECT: "bg-red-100 text-red-800",
    MAPREMOTE: "bg-purple-100 text-purple-800",
    MAPLOCAL: "bg-orange-100 text-orange-800",
  };

  return (
    <div className={`bg-white rounded-lg shadow p-4 ${!rule.enabled ? "opacity-50" : ""}`}>
      <div className="flex justify-between items-start mb-2">
        <div className="flex items-center gap-2">
          <button
            onClick={() => onToggle(!rule.enabled)}
            className={`w-8 h-5 rounded-full flex items-center justify-center transition-colors ${
              rule.enabled ? "bg-blue-600" : "bg-gray-300"
            }`}
          >
            <Check size={14} className="text-white" />
          </button>
          <h3 className="font-semibold text-gray-900">{rule.name || "Unnamed Rule"}</h3>
        </div>
        <span className={`px-2 py-1 rounded text-xs font-medium ${actionColors[rule.action] || "bg-gray-100"}`}>
          {rule.action}
        </span>
      </div>

      <div className="space-y-1 text-sm text-gray-600 mb-3">
        <p>
          <span className="font-medium">{rule.pattern}:</span> {rule.value}
        </p>
        {rule.comment && <p className="text-gray-500">{rule.comment}</p>}
      </div>

      <div className="flex justify-end gap-2">
        <button
          onClick={onEdit}
          className="p-2 text-gray-600 hover:text-blue-600 hover:bg-gray-100 rounded"
        >
          <Pencil size={16} />
        </button>
        <button
          onClick={onDelete}
          className="p-2 text-gray-600 hover:text-red-600 hover:bg-gray-100 rounded"
        >
          <Trash2 size={16} />
        </button>
      </div>
    </div>
  );
}
