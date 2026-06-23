import { useEffect } from "react";
import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { useUpdateCheck } from "../../hooks/useUpdateCheck";
import { BreakpointPanel } from "../breakpoint/BreakpointPanel";

export function Layout() {
  const { checkForUpdates } = useUpdateCheck();

  // Spec §2: App-startup background check for new releases.
  // Runs exactly once per app session (Layout is always mounted).
  // Errors are swallowed by useUpdateCheck — a failed check just leaves
  // `hasUpdate` false, which is the safe default.
  useEffect(() => {
    checkForUpdates();
  }, [checkForUpdates]);

  return (
    <div className="flex h-screen">
      <Sidebar />
      <main className="flex-1 overflow-auto bg-surface-primary p-6">
        <Outlet />
      </main>
      <BreakpointPanel />
    </div>
  );
}
