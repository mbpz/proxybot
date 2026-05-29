import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { RulesPage } from "../components/rules/RulesPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "list_rule_files") return Promise.resolve(["rules.yaml"]);
    if (cmd === "get_rules") return Promise.resolve([]);
    return Promise.resolve(null);
  }),
}));

describe("RulesPage", () => {
  it("shows empty state when no rules", async () => {
    render(<RulesPage />);
    expect(await screen.findByText("No rules configured")).toBeInTheDocument();
  });

  it("shows add rule button", async () => {
    render(<RulesPage />);
    expect(await screen.findByText("+ Add Rule")).toBeInTheDocument();
  });
});
