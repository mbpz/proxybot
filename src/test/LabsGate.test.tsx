import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it } from "vitest";
import { LabsGate } from "../features/labs/LabsGate";
import { LabsProvider, LABS_STORAGE_KEY } from "../features/labs/LabsContext";

function renderGate() {
  return render(
    <MemoryRouter>
      <LabsProvider>
        <LabsGate>
          <p>Labs content</p>
        </LabsGate>
      </LabsProvider>
    </MemoryRouter>,
  );
}

describe("LabsGate", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("blocks Labs routes by default with one consistent explanation", () => {
    renderGate();
    expect(screen.getByText("Labs are off")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Open Settings → Labs" })).toHaveAttribute(
      "href",
      "/settings",
    );
    expect(screen.queryByText("Labs content")).not.toBeInTheDocument();
  });

  it("renders the Labs surface once enabled", () => {
    window.localStorage.setItem(LABS_STORAGE_KEY, "true");
    renderGate();
    expect(screen.getByText("Labs content")).toBeInTheDocument();
    expect(screen.queryByText("Labs are off")).not.toBeInTheDocument();
  });
});
