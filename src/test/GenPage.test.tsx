import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { GenPage } from "../components/gen/GenPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_mock_endpoints") return Promise.resolve([]);
    if (cmd === "generate_mock_project") return Promise.resolve(null);
    if (cmd === "write_mock_project") return Promise.resolve("");
    if (cmd === "start_mock_server") return Promise.resolve("");
    if (cmd === "stop_mock_server") return Promise.resolve(null);
    if (cmd === "get_scaffold_template") return Promise.resolve(null);
    if (cmd === "generate_scaffold_project") return Promise.resolve(null);
    if (cmd === "write_scaffold_project") return Promise.resolve("");
    if (cmd === "evaluate_scaffold_project") return Promise.resolve([false, 0, []]);
    return Promise.resolve(null);
  }),
}));

describe("GenPage", () => {
  it("renders generator page with tabs", async () => {
    render(<GenPage />);
    expect(await screen.findByText("Generate")).toBeInTheDocument();
    expect(await screen.findByText("Mock API")).toBeInTheDocument();
    expect(await screen.findByText("Scaffold")).toBeInTheDocument();
  });
});
