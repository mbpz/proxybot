import { useEffect } from "react";
import { useSslBypass } from "../../stores/sslBypassStore";

export function ProcessList() {
  const store = useSslBypass();

  useEffect(() => {
    if (store.selectedDevice) {
      store.refreshProcesses();
    }
  }, [store.selectedDevice]);

  if (!store.selectedDevice) return null;

  return (
    <div className="card mb-4">
      <div className="flex justify-between items-center mb-2">
        <h3 className="card-title text-base">Processes</h3>
        <button
          onClick={store.refreshProcesses}
          className="btn btn-sm btn-secondary"
          data-testid="ssl-bypass-refresh-processes"
        >
          Refresh
        </button>
      </div>
      {store.processes.length === 0 ? (
        <p className="text-sm text-text-muted">No processes found.</p>
      ) : (
        <ul className="text-sm space-y-1 max-h-64 overflow-y-auto" data-testid="ssl-bypass-process-list">
          {store.processes.map((p) => (
            <li
              key={p.pid}
              className="flex justify-between items-center px-2 py-1 hover:bg-surface-elevated rounded"
            >
              <span>
                {p.name} <span className="text-text-muted">(PID: {p.pid})</span>
              </span>
              <button
                disabled={!store.selectedScript}
                onClick={() =>
                  store.selectedScript && store.injectScript(p.pid, store.selectedScript)
                }
                className="btn btn-sm btn-primary disabled:opacity-50"
                data-testid={`ssl-bypass-inject-${p.pid}`}
              >
                Inject
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}