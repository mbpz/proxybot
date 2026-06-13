import { useSslBypass } from "../../stores/sslBypassStore";

export function ScriptList() {
  const store = useSslBypass();
  return (
    <div className="card mb-4">
      <div className="flex justify-between items-center mb-2">
        <h3 className="card-title text-base">Bypass Scripts</h3>
        <button
          onClick={store.refreshScripts}
          className="btn btn-sm btn-secondary"
          data-testid="ssl-bypass-refresh-scripts"
        >
          Refresh
        </button>
      </div>
      {store.scripts.length === 0 ? (
        <p className="text-sm text-text-muted">No scripts available.</p>
      ) : (
        <div className="space-y-1" data-testid="ssl-bypass-script-list">
          {store.scripts.map((s) => (
            <button
              key={s.id}
              onClick={() => store.setSelectedScript(s.id)}
              className={`w-full text-left px-3 py-2 rounded border transition-colors ${
                store.selectedScript === s.id
                  ? "border-accent-blue bg-[rgba(0,212,255,0.08)]"
                  : "border-transparent hover:bg-surface-elevated"
              }`}
              data-testid={`ssl-bypass-script-${s.id}`}
            >
              <div className="flex justify-between items-center">
                <span className="font-medium text-sm">{s.name}</span>
                {s.is_builtin ? (
                  <span className="text-xs px-1.5 py-0.5 rounded bg-surface-elevated text-text-secondary">
                    built-in
                  </span>
                ) : (
                  <span className="text-xs px-1.5 py-0.5 rounded bg-accent-blue text-surface-primary">
                    custom
                  </span>
                )}
              </div>
              <p className="text-xs text-text-muted mt-0.5">{s.description}</p>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}