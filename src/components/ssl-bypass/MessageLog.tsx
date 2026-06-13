import { useEffect, useRef } from "react";
import { useSslBypass } from "../../stores/sslBypassStore";

/**
 * Scrollable live log of messages streamed from running Frida bypass
 * scripts via the `frida:message` Tauri event (spec §9.4).
 *
 * Auto-scrolls to the latest entry whenever a new message arrives.
 * Errors render in red; other levels render in the default muted
 * foreground. A Clear button wipes the buffer through the store.
 */
export function MessageLog() {
  const store = useSslBypass();
  const logRef = useRef<HTMLDivElement>(null);

  // Auto-scroll on new messages. Intentionally not throttled —
  // React batches state updates and a single reflow per render is
  // cheap for our 1000-entry cap.
  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [store.messages.length]);

  return (
    <div className="border rounded-lg p-3 bg-card text-card-foreground mt-4">
      <div className="flex justify-between items-center mb-2">
        <h2 className="text-sm font-semibold">Live Log</h2>
        <button
          type="button"
          onClick={store.clearMessages}
          disabled={store.messages.length === 0}
          className="text-xs px-2 py-0.5 rounded bg-muted hover:bg-muted/70 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Clear
        </button>
      </div>
      <div
        ref={logRef}
        className="h-32 overflow-y-auto bg-muted/30 rounded p-2 text-xs font-mono space-y-0.5"
        data-testid="frida-message-log"
      >
        {store.messages.length === 0 ? (
          <p className="text-muted-foreground">
            No messages yet. Inject a script to see output.
          </p>
        ) : (
          store.messages.map((msg, i) => (
            <div
              key={`${msg.timestamp_ms}-${i}`}
              className={msg.level === "error" ? "text-red-400" : ""}
            >
              <span className="text-muted-foreground">[{msg.level}]</span>{" "}
              {msg.payload}
            </div>
          ))
        )}
      </div>
    </div>
  );
}