import { FileText } from "lucide-react";

interface HeadersViewProps {
  headers: Record<string, string>;
}

export function HeadersView({ headers }: HeadersViewProps) {
  const headerEntries = Object.entries(headers);

  const copyHeaders = () => {
    const text = headerEntries.map(([k, v]) => `${k}: ${v}`).join("\n");
    navigator.clipboard.writeText(text);
  };

  if (headerEntries.length === 0) {
    return (
      <div className="empty-state">
        <FileText size={48} className="empty-state-icon" />
        <div className="empty-state-title">No headers</div>
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between px-4 py-2 bg-surface-tertiary border-b border-border">
        <span className="text-xs text-secondary font-mono uppercase">
          {headerEntries.length} headers
        </span>
        <button onClick={copyHeaders} className="btn btn-ghost btn-sm">
          Copy
        </button>
      </div>

      <table className="table">
        <tbody>
          {headerEntries.map(([key, value]) => (
            <tr key={key}>
              <td className="font-mono text-accent-blue w-[30%] !py-2 !px-3">
                {key}
              </td>
              <td className="font-mono text-text-primary break-all !py-2 !px-3">
                {value}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
