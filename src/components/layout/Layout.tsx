import { useEffect } from "react";
import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { useUpdateCheck } from "../../hooks/useUpdateCheck";
import { BreakpointPanel } from "../breakpoint/BreakpointPanel";
import {
  CaptureSessionBar,
  CaptureSessionProvider,
} from "../../features/capture-session/CaptureSession";

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
    <CaptureSessionProvider>
      <div className="flex h-screen">
        <Sidebar />
        <section className="flex min-w-0 flex-1 flex-col">
          <CaptureSessionBar />
          <main className="flex-1 overflow-auto bg-surface-primary p-6">
            <Outlet />
          </main>
        </section>
        <BreakpointPanel />
      </div>
    </CaptureSessionProvider>
  );
}
