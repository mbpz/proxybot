import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { ComposerPage } from "../components/composer/ComposerPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "compose_request")
      return Promise.resolve({ status: 200, headers: {}, body: "", duration_ms: 0 });
    return Promise.resolve(null);
  }),
}));

describe("ComposerPage", () => {
  it("renders composer page", async () => {
    render(<ComposerPage />);
    expect(await screen.findByText("Composer")).toBeInTheDocument();
    expect(await screen.findByText("Response")).toBeInTheDocument();
  });
});
