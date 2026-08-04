import { beforeEach, describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { MemoryRouter } from "react-router-dom";
import { CertsPage } from "../components/certs/CertsPage";

function defaultInvoke(cmd: string) {
  if (cmd === "get_ca_metadata") return Promise.resolve(null);
  return Promise.resolve(null);
}

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(defaultInvoke),
}));

describe("CertsPage", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockClear();
    vi.mocked(invoke).mockImplementation(defaultInvoke);
  });

  it("renders certificates page", async () => {
    render(<MemoryRouter><CertsPage /></MemoryRouter>);
    expect(await screen.findByText("Advanced Certificates")).toBeInTheDocument();
    expect(await screen.findByText("Root CA Certificate")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Back to Device Setup" })).toHaveAttribute(
      "href",
      "/setup",
    );
    expect(screen.queryByRole("button", { name: /CA Server/ })).not.toBeInTheDocument();
  });
});
