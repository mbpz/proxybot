import { Link } from "react-router-dom";
import { FlaskConical } from "lucide-react";
import { useLabs } from "../../features/labs/LabsContext";

const LABS_PAGES = [
  { path: "/graph", label: "Graph" },
  { path: "/topology", label: "Topology" },
  { path: "/ai", label: "AI Analysis" },
  { path: "/gen", label: "Generate" },
  { path: "/deploy", label: "Deploy" },
];

/** Opt-in switch for experimental surfaces; persisted locally only. */
export function LabsTab() {
  const { enabled, setEnabled } = useLabs();

  return (
    <div className="space-y-4">
      <div className="card flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <FlaskConical size={20} className="text-accent-purple" />
          <div>
            <div className="font-medium text-text-primary">Enable Labs</div>
            <div className="text-sm text-text-muted">
              Experimental analysis, generation and deployment surfaces. They are hidden from
              the default navigation, may change without notice, and are not part of the
              supported first-capture journey.
            </div>
          </div>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={enabled}
          aria-label="Enable Labs"
          onClick={() => setEnabled(!enabled)}
          className={`relative h-6 w-12 shrink-0 rounded-full transition-colors ${
            enabled ? "bg-accent-purple" : "bg-surface-tertiary"
          }`}
          data-testid="labs-toggle"
        >
          <span
            className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition-transform ${
              enabled ? "translate-x-6" : "translate-x-0.5"
            }`}
          />
        </button>
      </div>

      {enabled ? (
        <div className="card">
          <div className="mb-3 font-medium text-text-primary">Available Labs</div>
          <div className="flex flex-wrap gap-2">
            {LABS_PAGES.map((page) => (
              <Link key={page.path} to={page.path} className="btn btn-secondary btn-sm">
                {page.label}
              </Link>
            ))}
          </div>
          <p className="mt-3 text-xs text-text-muted">
            You can also reach these pages from the command palette (⌘K).
          </p>
        </div>
      ) : (
        <div className="card text-sm text-text-muted">
          Labs are off. Opening an experimental route shows a single "Labs are off" page with a
          link back here.
        </div>
      )}
    </div>
  );
}
