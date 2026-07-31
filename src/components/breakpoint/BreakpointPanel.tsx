import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Ban, Play, X as XIcon } from "lucide-react";
import { Button } from "../ui/Button";
import { safeInvoke } from "../../utils/safeInvoke";

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
 * Docked panel showing pending breakpoints. The proxy rules engine
 * emits a `BreakpointRequest` when a rule matches; this panel reads
 * from the Tauri event, fetches the snapshot, and lets the user
 * forward / edit-and-forward / drop each one.
 */
export function BreakpointPanel() {
  const [pending, setPending] = useState<BreakpointSnapshot[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  // Listen for new breakpoints from the Rust side.
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    (async () => {
      unlisten = await listen("breakpoint:new", () => {
        refresh();
      });
    })();
    // Initial load.
    refresh();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  async function refresh() {
    const result = await safeInvoke<unknown>("get_pending_breakpoints");
    const list = Array.isArray(result)
      ? (result as BreakpointSnapshot[])
      : [];

    setPending(list);
    setSelectedId((current) =>
      current && !list.some((breakpoint) => breakpoint.id === current)
        ? null
        : current
    );
  }

  const resolve = useCallback(
    async (id: string, decision: Decision) => {
      try {
        await invoke<number>("resolve_breakpoint", {
          id,
          decision,
          mutated: null,
        });
        refresh();
      } catch (err) {
        console.error("resolve_breakpoint failed:", err);
      }
    },
    []
  );

  const selected = pending.find((b) => b.id === selectedId);
  const tsxRef = useCallback(
    (node: HTMLTextAreaElement | null) => {
      if (node) node.focus();
    },
    []
  );

  if (pending.length === 0) return null;

  return (
    <div className="fixed bottom-0 right-4 w-96 max-h-[70vh] bg-white dark:bg-slate-900 border border-slate-300 dark:border-slate-700 rounded-t-lg shadow-xl z-50 flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-slate-200 dark:border-slate-800">
        <span className="text-sm font-semibold flex items-center gap-1.5">
          <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse" />
          Breakpoints ({pending.length})
        </span>
        <button
          className="text-text-muted hover:text-text"
          onClick={() => {
            invoke("cancel_all_breakpoints").catch(() => {});
            setPending([]);
          }}
          aria-label="Cancel all breakpoints"
        >
          <XIcon size={14} />
        </button>
      </div>

      {/* List */}
      <ul className="overflow-y-auto flex-1 divide-y divide-slate-100 dark:divide-slate-800 text-xs">
        {pending.map((bp) => (
          <li
            key={bp.id}
            className={`px-3 py-2 cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/50 ${
              selectedId === bp.id
                ? "bg-slate-100 dark:bg-slate-800"
                : ""
            }`}
            onClick={() => setSelectedId(bp.id)}
          >
            <div className="flex items-center gap-2">
              <span
                className={`font-mono font-semibold ${
                  bp.target === "Request"
                    ? "text-blue-600"
                    : bp.target === "Response"
                    ? "text-green-600"
                    : "text-amber-600"
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
        <div className="border-t border-slate-200 dark:border-slate-800 px-3 py-2 space-y-2 max-h-64 overflow-y-auto">
          <div className="text-xs font-semibold flex items-center gap-2">
            <span>Edit request</span>
            <span className="text-text-muted font-normal">
              {selected.target} breakpoint
            </span>
          </div>

          {/* Method + Path */}
          <div className="flex gap-2">
            <select
              className="input font-mono text-xs w-24"
              defaultValue={selected.request.method}
              disabled
            >
              <option>GET</option>
              <option>POST</option>
              <option>PUT</option>
              <option>DELETE</option>
              <option>PATCH</option>
            </select>
            <input
              className="input flex-1 font-mono text-xs"
              defaultValue={selected.request.path}
              disabled
            />
          </div>

          {/* Body preview / edit */}
          <textarea
            ref={tsxRef}
            className="input w-full font-mono text-xs h-20 resize-none"
            defaultValue={
              selected.request.req_body ?? "// no body"
            }
            readOnly
          />

          {/* Action buttons */}
          <div className="flex gap-2">
            <Button
              variant="secondary"
              size="sm"
              className="flex-1"
              onClick={() => resolve(selected.id, "proceed")}
            >
              <Play size={14} />
              Forward
            </Button>
            <Button
              variant="danger"
              size="sm"
              className="flex-1"
              onClick={() => resolve(selected.id, "drop")}
            >
              <Ban size={14} />
              Drop
            </Button>
          </div>
          <p className="text-[10px] text-text-muted">
            Edit-and-forward ("modify") is not yet wired — selecting
            "Forward" or "Drop" resolves this breakpoint.
          </p>
        </div>
      )}
    </div>
  );
}
