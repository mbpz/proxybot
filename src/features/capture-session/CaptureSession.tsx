import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type PropsWithChildren,
} from "react";
import { Link } from "react-router-dom";
import { AlertCircle, KeyRound, Play, RefreshCw, Square, X } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { desktop, type DesktopContract } from "../../desktop/contract";

type CaptureOperation = "checking" | "starting" | "stopping" | null;

interface CaptureSessionState {
  running: boolean;
  initialized: boolean;
  operation: CaptureOperation;
  error: string | null;
}

interface CaptureSessionValue extends CaptureSessionState {
  start(): Promise<void>;
  stop(): Promise<void>;
  refresh(): Promise<void>;
  dismissError(): void;
}

interface CaptureSessionProviderProps extends PropsWithChildren {
  contract?: DesktopContract;
}

const INITIAL_STATE: CaptureSessionState = {
  running: false,
  initialized: false,
  operation: "checking",
  error: null,
};

const CaptureSessionContext = createContext<CaptureSessionValue | null>(null);

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function CaptureSessionProvider({
  children,
  contract = desktop,
}: CaptureSessionProviderProps) {
  const [state, setState] = useState<CaptureSessionState>(INITIAL_STATE);

  const refresh = useCallback(async () => {
    setState((current) => ({
      ...current,
      operation: current.initialized ? current.operation : "checking",
    }));
    try {
      const running = await contract.call("get_proxy_status", {});
      setState({ running, initialized: true, operation: null, error: null });
    } catch (error) {
      setState((current) => ({
        ...current,
        initialized: true,
        operation: null,
        error: errorMessage(error),
      }));
    }
  }, [contract]);

  const reconcileFailure = useCallback(
    async (error: unknown) => {
      let running = false;
      try {
        running = await contract.call("get_proxy_status", {});
      } catch {
        // Preserve the original lifecycle failure. Refresh remains available.
      }
      setState({
        running,
        initialized: true,
        operation: null,
        error: errorMessage(error),
      });
    },
    [contract],
  );

  const start = useCallback(async () => {
    setState((current) => ({ ...current, operation: "starting", error: null }));
    try {
      await contract.call("start_proxy", {});
      setState({ running: true, initialized: true, operation: null, error: null });
    } catch (error) {
      await reconcileFailure(error);
    }
  }, [contract, reconcileFailure]);

  const stop = useCallback(async () => {
    setState((current) => ({ ...current, operation: "stopping", error: null }));
    try {
      await contract.call("stop_proxy", {});
      setState({ running: false, initialized: true, operation: null, error: null });
    } catch (error) {
      await reconcileFailure(error);
    }
  }, [contract, reconcileFailure]);

  const dismissError = useCallback(() => {
    setState((current) => ({ ...current, error: null }));
  }, []);

  useEffect(() => {
    const subscription = contract.subscribe("capture-session:changed", {
      next: (running) => {
        setState({ running, initialized: true, operation: null, error: null });
      },
      error: (error) => {
        setState((current) => ({ ...current, error: error.message }));
      },
    });
    void subscription.ready.catch((error) => {
      setState((current) => ({ ...current, error: errorMessage(error) }));
    });
    void refresh();

    const refreshOnFocus = () => void refresh();
    window.addEventListener("focus", refreshOnFocus);
    return () => {
      window.removeEventListener("focus", refreshOnFocus);
      subscription.dispose();
    };
  }, [contract, refresh]);

  const value = useMemo<CaptureSessionValue>(
    () => ({ ...state, start, stop, refresh, dismissError }),
    [dismissError, refresh, start, state, stop],
  );

  return (
    <CaptureSessionContext.Provider value={value}>
      {children}
    </CaptureSessionContext.Provider>
  );
}

export function useCaptureSession(): CaptureSessionValue {
  const value = useContext(CaptureSessionContext);
  if (!value) {
    throw new Error("useCaptureSession must be used within CaptureSessionProvider");
  }
  return value;
}

export function CaptureSessionBar() {
  const session = useCaptureSession();
  const busy = session.operation !== null;
  const status =
    session.operation === "checking"
      ? "Checking capture status"
      : session.operation === "starting"
        ? "Starting capture"
        : session.operation === "stopping"
          ? "Stopping capture"
          : session.running
            ? "Capturing"
            : "Capture stopped";

  return (
    <div className="border-b border-border bg-surface-secondary">
      <div className="flex min-h-14 items-center justify-between gap-4 px-6 py-2">
        <div className="flex min-w-0 items-center gap-3" role="status" aria-live="polite">
          <span
            className={`h-2.5 w-2.5 shrink-0 rounded-full ${
              session.running ? "bg-accent-green" : "bg-text-muted"
            }`}
            aria-hidden="true"
          />
          <div className="min-w-0">
            <div className="font-medium text-text-primary">{status}</div>
            <div className="truncate text-xs text-text-muted">
              {session.running
                ? "Requests from configured devices will appear in Traffic."
                : "Start capture, then connect a test device with an explicit proxy."}
            </div>
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <Link
            to="/setup"
            className="btn btn-secondary btn-sm"
            aria-label="Open device setup"
          >
            <KeyRound size={15} />
            Setup
          </Link>
          {session.running ? (
            <Button variant="danger" size="sm" onClick={() => void session.stop()} disabled={busy}>
              <Square size={14} />
              {session.operation === "stopping" ? "Stopping..." : "Stop Capture"}
            </Button>
          ) : (
            <Button variant="primary" size="sm" onClick={() => void session.start()} disabled={busy}>
              <Play size={15} />
              {session.operation === "starting" ? "Starting..." : "Start Capture"}
            </Button>
          )}
        </div>
      </div>

      {session.error && (
        <div className="flex items-center gap-2 border-t border-red-500/30 bg-red-500/10 px-6 py-2 text-sm text-red-300" role="alert">
          <AlertCircle size={16} className="shrink-0" />
          <span className="min-w-0 flex-1 truncate">{session.error}</span>
          <Button variant="ghost" size="sm" onClick={() => void session.refresh()}>
            <RefreshCw size={14} />
            Retry
          </Button>
          <button
            type="button"
            className="rounded p-1 hover:bg-red-500/20"
            onClick={session.dismissError}
            aria-label="Dismiss capture error"
          >
            <X size={15} />
          </button>
        </div>
      )}
    </div>
  );
}
