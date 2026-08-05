import { Outlet } from "react-router-dom";
import { ContextNav, type ContextNavItem } from "../../components/layout/ContextNav";

const replayTools: readonly ContextNavItem[] = [
  { path: "/replay", label: "Replay" },
  { path: "/composer", label: "Composer" },
];

/** Request execution tools kept behind one default Replay destination. */
export function ReplayWorkspace() {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <ContextNav label="Replay tools" items={replayTools} />
      <div className="min-h-0 flex-1">
        <Outlet />
      </div>
    </div>
  );
}
