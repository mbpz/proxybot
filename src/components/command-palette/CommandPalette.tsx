import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Dialog } from "../ui/Dialog";
import { InlineNotice } from "../ui/states";
import { useCaptureSession } from "../../features/capture-session/CaptureSession";
import { useLabs } from "../../features/labs/LabsContext";
import { desktop } from "../../desktop/contract";

export const OPEN_COMMAND_PALETTE_EVENT = "proxybot:command-palette:open";

export function openCommandPalette(): void {
  window.dispatchEvent(new Event(OPEN_COMMAND_PALETTE_EVENT));
}

interface PaletteCommand {
  id: string;
  label: string;
  hint?: string;
  keywords?: string;
  run: () => void | Promise<void>;
}

/** Cmd+K / Ctrl+K command palette: navigate, capture lifecycle, HAR export. */
export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<{
    variant: "info" | "success" | "error";
    message: string;
  } | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const navigate = useNavigate();
  const capture = useCaptureSession();
  const labs = useLabs();

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setOpen((current) => !current);
      }
    }
    function onOpenRequest() {
      setOpen(true);
    }
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener(OPEN_COMMAND_PALETTE_EVENT, onOpenRequest);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener(OPEN_COMMAND_PALETTE_EVENT, onOpenRequest);
    };
  }, []);

  useEffect(() => {
    if (open) {
      setQuery("");
      setActiveIndex(0);
      setNotice(null);
    }
  }, [open]);

  const commands = useMemo<PaletteCommand[]>(() => {
    const nav = (path: string, label: string): PaletteCommand => ({
      id: `nav:${path}`,
      label: `Go to ${label}`,
      hint: path,
      keywords: `${label} ${path} navigate`,
      run: () => {
        navigate(path);
        setOpen(false);
      },
    });

    const items: PaletteCommand[] = [
      nav("/", "Capture"),
      nav("/setup", "Setup"),
      nav("/settings", "Settings"),
      nav("/rules", "Rules"),
      nav("/replay", "Replay"),
      nav("/composer", "Composer"),
      nav("/dns", "DNS"),
      nav("/alerts", "Alerts"),
      nav("/certs", "Certificates"),
      nav("/devices", "Devices"),
    ];

    if (labs.enabled) {
      items.push(
        nav("/graph", "Graph (Labs)"),
        nav("/topology", "Topology (Labs)"),
        nav("/ai", "AI Analysis (Labs)"),
        nav("/gen", "Generate (Labs)"),
        nav("/deploy", "Deploy (Labs)"),
      );
    }

    items.push({
      id: "capture:toggle",
      label: capture.running ? "Stop Capture Session" : "Start Capture Session",
      hint: "Capture",
      keywords: "capture start stop proxy session",
      run: async () => {
        const wasRunning = capture.running;
        setBusy(true);
        try {
          if (wasRunning) {
            await capture.stop();
            setNotice({ variant: "success", message: "Capture stopped" });
          } else {
            await capture.start();
            setNotice({ variant: "success", message: "Capture started" });
          }
        } catch (error) {
          setNotice({ variant: "error", message: String(error) });
        } finally {
          setBusy(false);
        }
      },
    });

    items.push({
      id: "har:export",
      label: "Export HAR",
      hint: "Capture",
      keywords: "har export save download",
      run: async () => {
        setBusy(true);
        try {
          const sessionName = `proxybot-${new Date().toISOString().replace(/[:.]/g, "-")}`;
          const har = await desktop.call("export_har", { sessionName });
          const path = await desktop.call("save_har_file", {
            harJson: JSON.stringify(har),
            sessionName,
          });
          setNotice({ variant: "success", message: `HAR exported to ${path}` });
        } catch (error) {
          setNotice({ variant: "error", message: String(error) });
        } finally {
          setBusy(false);
        }
      },
    });

    return items;
  }, [capture, labs.enabled, navigate]);

  const filtered = useMemo(() => {
    const term = query.trim().toLowerCase();
    if (!term) return commands;
    return commands.filter((command) =>
      `${command.label} ${command.hint ?? ""} ${command.keywords ?? ""}`.toLowerCase().includes(term),
    );
  }, [commands, query]);

  useEffect(() => {
    setActiveIndex((current) => Math.min(current, Math.max(filtered.length - 1, 0)));
  }, [filtered.length]);

  function runCommand(command: PaletteCommand) {
    void command.run();
  }

  function onInputKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((current) => Math.min(current + 1, filtered.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((current) => Math.max(current - 1, 0));
    } else if (event.key === "Enter") {
      event.preventDefault();
      const command = filtered[activeIndex];
      if (command) runCommand(command);
    }
  }

  return (
    <>
      <Dialog
        open={open}
        title="Command palette (⌘K)"
        onClose={() => setOpen(false)}
        initialFocusRef={inputRef}
      >
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={onInputKeyDown}
          placeholder="Search destinations, actions..."
          aria-label="Search commands"
          className="w-full"
          data-testid="command-palette-input"
        />
        {notice && (
          <div className="mt-3">
            <InlineNotice variant={notice.variant} onDismiss={() => setNotice(null)}>
              {notice.message}
            </InlineNotice>
          </div>
        )}
        <ul role="listbox" aria-label="Commands" className="mt-3 max-h-80 overflow-auto">
          {filtered.map((command, index) => (
            <li key={command.id} role="option" aria-selected={index === activeIndex}>
              <button
                type="button"
                onClick={() => runCommand(command)}
                onMouseEnter={() => setActiveIndex(index)}
                className={`flex w-full items-center justify-between gap-3 rounded px-3 py-2 text-left text-sm ${
                  index === activeIndex
                    ? "bg-surface-tertiary text-text-primary"
                    : "text-text-secondary hover:bg-surface-tertiary"
                }`}
              >
                <span>{command.label}</span>
                {command.hint && <span className="text-xs text-text-muted">{command.hint}</span>}
              </button>
            </li>
          ))}
          {filtered.length === 0 && (
            <li className="px-3 py-2 text-sm text-text-muted">No matching commands</li>
          )}
        </ul>
        {busy && <p className="mt-2 text-xs text-text-muted">Working...</p>}
        <p className="mt-3 text-xs text-text-muted">
          ↑↓ select · Enter run · Esc close
        </p>
      </Dialog>
    </>
  );
}
