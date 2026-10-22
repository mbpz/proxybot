import { Outlet } from "react-router-dom";
import { ContextNav, type ContextNavItem } from "../../components/layout/ContextNav";
import { useLabs } from "../labs/LabsContext";

/**
 * Request-derived workbench. Tools that used to occupy the default sidebar
 * (Rules, Replay, DNS, Alerts, Analysis) stay one click from Capture instead.
 * Labs destinations are reachable from the command palette once opted in.
 */
const captureViews: readonly ContextNavItem[] = [
  { path: "/", label: "Requests", end: true },
  { path: "/rules", label: "Rules" },
  { path: "/replay", label: "Replay" },
  { path: "/dns", label: "DNS" },
  { path: "/alerts", label: "Alerts" },
];

/** Analysis is a Labs capability and only appears once Labs are enabled. */
const labsCaptureViews: readonly ContextNavItem[] = [{ path: "/graph", label: "Analysis" }];

/** Requests and request-derived tools that belong to one Capture Session. */
export function CaptureWorkspace() {
  const { enabled } = useLabs();
  const items = enabled ? [...captureViews, ...labsCaptureViews] : captureViews;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <ContextNav label="Capture views" items={items} />
      <div className="min-h-0 flex-1">
        <Outlet />
      </div>
    </div>
  );
}
