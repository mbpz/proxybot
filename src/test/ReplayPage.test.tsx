import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { ReplayPage } from "../components/replay/ReplayPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_replay_targets") return Promise.resolve([]);
    if (cmd === "get_replay_results") return Promise.resolve([]);
    return Promise.resolve(null);
  }),
}));

describe("ReplayPage", () => {
  it("renders replay panel and shows empty state", async () => {
    render(<ReplayPage />);
    expect(await screen.findByText("Replay")).toBeInTheDocument();
    expect(await screen.findByText("No replay targets")).toBeInTheDocument();
  });
});
