import { render, screen, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { CaptureWorkspace } from "../../features/capture-session/CaptureWorkspace";
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
  it("exposes exactly five default destinations", () => {
    const sidebar = renderSidebar("/graph");
    expect(within(sidebar).getAllByRole("link").map((link) => link.textContent)).toEqual([
      "Capture",
      "Setup",
      "Rules",
      "Replay",
      "Settings",
    ]);
    expect(within(sidebar).getByRole("link", { name: "Capture" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("keeps Composer under the Replay destination", () => {
    const sidebar = renderSidebar("/composer");
    expect(within(sidebar).getByRole("link", { name: "Replay" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(within(sidebar).queryByRole("link", { name: "Composer" })).not.toBeInTheDocument();
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
    expect(within(screen.getByRole("navigation", { name: "Capture views" })).getAllByRole("link").map((link) => link.textContent)).toEqual([
      "Requests",
      "DNS",
      "Alerts",
      "Graph",
      "Topology",
    ]);
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
    expect(within(screen.getByRole("navigation", { name: "Replay tools" })).getAllByRole("link").map((link) => link.textContent)).toEqual([
      "Replay",
      "Composer",
    ]);
    expect(screen.getByText("Composer content")).toBeInTheDocument();
  });
});
