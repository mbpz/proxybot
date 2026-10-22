import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { FlaskConical } from "lucide-react";
import { useLabs } from "./LabsContext";

/** Blocks Labs routes with one consistent explanation and a path to opt in. */
export function LabsGate({ children }: { children: ReactNode }) {
  const { enabled } = useLabs();
  if (enabled) return <>{children}</>;
  return <LabsDisabledPage />;
}

function LabsDisabledPage() {
  return (
    <div className="mx-auto flex h-full max-w-xl flex-col items-center justify-center gap-4 text-center">
      <FlaskConical size={44} className="text-accent-purple opacity-70" aria-hidden="true" />
      <h1 className="text-2xl font-bold">Labs are off</h1>
      <p className="max-w-md text-sm text-text-secondary">
        AI analysis, Generate, Deploy, Graph, and Topology are experimental and
        off by default. Enable Labs under Settings → Labs, then open these pages
        from Settings or the command palette (⌘K).
      </p>
      <Link to="/settings" className="btn btn-primary">
        Open Settings → Labs
      </Link>
    </div>
  );
}
