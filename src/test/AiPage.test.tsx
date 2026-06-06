import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { AiPage } from "../components/ai/AiPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_ai_stats")
      return Promise.resolve({ stats: [], totals: { total_input_tokens: 0, total_output_tokens: 0 } });
    if (cmd === "get_ai_context_windows") return Promise.resolve({});
    if (cmd === "get_inferred_apis") return Promise.resolve([]);
    if (cmd === "get_auth_state_machine") return Promise.resolve(null);
    if (cmd === "get_vision_analyses") return Promise.resolve([]);
    return Promise.resolve(null);
  }),
}));

describe("AiPage", () => {
  it("renders AI Analysis page with tabs", async () => {
    render(<AiPage />);
    expect(await screen.findByText("AI Analysis")).toBeInTheDocument();
    expect(await screen.findByText("Token Usage")).toBeInTheDocument();
    expect(await screen.findByText("API Inference")).toBeInTheDocument();
  });
});
