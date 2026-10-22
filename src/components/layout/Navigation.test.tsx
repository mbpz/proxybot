import { render, screen, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { CaptureWorkspace } from "../../features/capture-session/CaptureWorkspace";
import { LabsProvider } from "../../features/labs/LabsContext";
import { ReplayWorkspace } from "../../features/replay-workspace/ReplayWorkspace";
import { Sidebar } from "./Sidebar";

function renderSidebar(path: string) {
  render(
    <MemoryRouter initialEntries={[path]}>
      <Sidebar />
    </MemoryRouter>,
  );
  return screen.getByRole("complementary");
}

describe("product navigation", () => {
  it("exposes exactly three default destinations", () => {
    const sidebar = renderSidebar("/graph");
    expect(within(sidebar).getAllByRole("link").map((link) => link.textContent)).toEqual([
      "Capture",
      "Setup",
      "Settings",
    ]);
    expect(within(sidebar).getByRole("link", { name: "Capture" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("keeps Composer inside the Capture destination without a sidebar entry", () => {
    const sidebar = renderSidebar("/composer");
    expect(within(sidebar).getByRole("link", { name: "Capture" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(within(sidebar).queryByRole("link", { name: "Composer" })).not.toBeInTheDocument();
    expect(within(sidebar).queryByRole("link", { name: "Replay" })).not.toBeInTheDocument();
  });

  it("places request-derived tools inside Capture and Replay contexts", () => {
    const { unmount } = render(
      <MemoryRouter initialEntries={["/dns"]}>
        <Routes>
          <Route element={<CaptureWorkspace />}>
            <Route path="dns" element={<p>DNS content</p>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );
    expect(
      within(screen.getByRole("navigation", { name: "Capture views" }))
        .getAllByRole("link")
        .map((link) => link.textContent),
    ).toEqual(["Requests", "Rules", "Replay", "DNS", "Alerts"]);
    expect(screen.getByText("DNS content")).toBeInTheDocument();

    unmount();
    render(
      <MemoryRouter initialEntries={["/composer"]}>
        <Routes>
          <Route element={<ReplayWorkspace />}>
            <Route path="composer" element={<p>Composer content</p>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );
    expect(
      within(screen.getByRole("navigation", { name: "Replay tools" }))
        .getAllByRole("link")
        .map((link) => link.textContent),
    ).toEqual(["Replay", "Composer"]);
    expect(screen.getByText("Composer content")).toBeInTheDocument();
  });

  it("adds Analysis to the Capture context only after Labs opt-in", () => {
    window.localStorage.setItem("proxybot.labs.enabled", "true");
    const { unmount } = render(
      <LabsProvider>
        <MemoryRouter initialEntries={["/dns"]}>
          <Routes>
            <Route element={<CaptureWorkspace />}>
              <Route path="dns" element={<p>DNS content</p>} />
            </Route>
          </Routes>
        </MemoryRouter>
      </LabsProvider>,
    );
    expect(
      within(screen.getByRole("navigation", { name: "Capture views" }))
        .getAllByRole("link")
        .map((link) => link.textContent),
    ).toEqual(["Requests", "Rules", "Replay", "DNS", "Alerts", "Analysis"]);
    unmount();
    window.localStorage.clear();
  });
});
