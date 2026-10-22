import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Ban, Play, X as XIcon } from "lucide-react";
import { Button } from "../ui/Button";

/** Wire mirror of `state::BreakpointSnapshot` on the Rust side. */
interface BreakpointSnapshot {
  id: string;
  target: "Request" | "Response" | "Both";
  request: {
    id: string;
    method: string;
    host: string;
    path: string;
    req_headers?: [string, string][];
    req_body?: string | null;
    scheme?: string;
    status?: number | null;
  };
}

/** Mirrors `proxy::BreakpointDecision` in Rust. */
type Decision = "proceed" | "drop" | "modify";

/**
 * Docked panel showing pending breakpoints. Forward/Drop are wired;
 * edit-and-forward ("modify") is not implemented yet and is stated in the UI.
 * Command failures surface here instead of silently hiding the panel.
 */
export function BreakpointPanel() {
  const [pending, setPending] = useState<BreakpointSnapshot[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const result = await invoke<unknown>("get_pending_breakpoints");
      const list = Array.isArray(result) ? (result as BreakpointSnapshot[]) : [];
      setError(null);
      setPending(list);
      setSelectedId((current) =>
        current && !list.some((breakpoint) => breakpoint.id === current) ? null : current,
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  // Listen for new breakpoints from the Rust side.
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    (async () => {
      unlisten = await listen("breakpoint:new", () => {
        void refresh();
      });
    })();
    void refresh();
    return () => {
      if (unlisten) unlisten();
    };
  }, [refresh]);

  const resolve = useCallback(
    async (id: string, decision: Decision) => {
      try {
        setError(null);
        await invoke<number>("resolve_breakpoint", {
          id,
          decision,
          mutated: null,
        });
        await refresh();
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : String(cause));
      }
    },
    [refresh],
  );

  const selected = pending.find((b) => b.id === selectedId);

  if (pending.length === 0 && !error) return null;

  return (
    <div className="fixed bottom-0 right-4 z-50 flex max-h-[70vh] w-96 flex-col rounded-t-lg border border-border bg-surface-secondary text-text-primary shadow-xl">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <span className="text-sm font-semibold flex items-center gap-1.5">
          <span className="h-2 w-2 rounded-full bg-accent-yellow animate-pulse" />
          Breakpoints ({pending.length})
        </span>
        <button
          type="button"
          className="rounded p-1 text-text-muted hover:bg-surface-tertiary hover:text-text-primary"
          onClick={() => {
            invoke("cancel_all_breakpoints")
              .then(() => setPending([]))
              .catch((cause) => setError(String(cause)));
          }}
          aria-label="Cancel all breakpoints"
        >
          <XIcon size={14} />
        </button>
      </div>

      {error && (
        <div className="m-2">
          <div className="error-banner" role="alert">
            <span className="error-banner-message">Breakpoints: {error}</span>
            <Button variant="secondary" size="sm" onClick={() => void refresh()}>
              Retry
            </Button>
          </div>
        </div>
      )}

      {/* List */}
      <ul className="flex-1 divide-y divide-border overflow-y-auto text-xs" role="listbox" aria-label="Pending breakpoints">
        {pending.map((bp) => (
          <li
            key={bp.id}
            role="option"
            aria-selected={selectedId === bp.id}
            tabIndex={0}
            className={`cursor-pointer px-3 py-2 outline-none focus-visible:ring-1 focus-visible:ring-accent-blue ${
              selectedId === bp.id ? "bg-surface-tertiary" : "hover:bg-surface-tertiary/60"
            }`}
            onClick={() => setSelectedId(bp.id)}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                setSelectedId(bp.id);
              }
            }}
          >
            <div className="flex items-center gap-2">
              <span
                className={`font-mono font-semibold ${
                  bp.target === "Request"
                    ? "text-accent-blue"
                    : bp.target === "Response"
                      ? "text-accent-green"
                      : "text-accent-yellow"
                }`}
              >
                {bp.request.method}
              </span>
              <span className="flex-1 truncate text-text-muted">
                {bp.request.host}
                {bp.request.path}
              </span>
            </div>
          </li>
        ))}
      </ul>

      {/* Detail panel */}
      {selected && (
        <div className="max-h-64 space-y-2 overflow-y-auto border-t border-border px-3 py-2">
          <div className="flex items-center gap-2 text-xs font-semibold">
            <span>Breakpoint request</span>
            <span className="font-normal text-text-muted">{selected.target}</span>
          </div>

          <div className="flex gap-2">
            <label className="sr-only" htmlFor="breakpoint-method">
              Method
            </label>
            <select
              id="breakpoint-method"
              className="input w-24 font-mono text-xs"
              defaultValue={selected.request.method}
              disabled
            >
              <option>GET</option>
              <option>POST</option>
              <option>PUT</option>
              <option>DELETE</option>
              <option>PATCH</option>
            </select>
            <label className="sr-only" htmlFor="breakpoint-path">
              Path
            </label>
            <input
              id="breakpoint-path"
              className="input flex-1 font-mono text-xs"
              defaultValue={selected.request.path}
              disabled
            />
          </div>

          <label className="sr-only" htmlFor="breakpoint-body">
            Request body
          </label>
          <textarea
            id="breakpoint-body"
            className="input h-20 w-full resize-none font-mono text-xs"
            defaultValue={selected.request.req_body ?? "// no body"}
            readOnly
          />

          <div className="flex gap-2">
            <Button
              variant="secondary"
              size="sm"
              className="flex-1"
              onClick={() => void resolve(selected.id, "proceed")}
            >
              <Play size={14} />
              Forward
            </Button>
            <Button
              variant="danger"
              size="sm"
              className="flex-1"
              onClick={() => void resolve(selected.id, "drop")}
            >
              <Ban size={14} />
              Drop
            </Button>
          </div>
          <p className="text-[10px] text-text-muted">
            Edit-and-forward ("modify") is not wired yet — Forward or Drop resolves this
            breakpoint. Use the matching Routing Rule to change future requests.
          </p>
        </div>
      )}
    </div>
  );
}
