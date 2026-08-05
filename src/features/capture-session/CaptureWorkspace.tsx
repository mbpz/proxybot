import { Outlet } from "react-router-dom";
import { ContextNav, type ContextNavItem } from "../../components/layout/ContextNav";

const captureViews: readonly ContextNavItem[] = [
  { path: "/", label: "Requests", end: true },
  { path: "/dns", label: "DNS" },
  { path: "/alerts", label: "Alerts" },
  { path: "/graph", label: "Graph" },
  { path: "/topology", label: "Topology" },
];

/** Requests and request-derived tools that belong to one Capture Session. */
export function CaptureWorkspace() {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <ContextNav label="Capture views" items={captureViews} />
      <div className="min-h-0 flex-1">
        <Outlet />
      </div>
    </div>
  );
}
