import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Rule } from "../generated/desktop-contract";
import { RulesPage } from "../components/rules/RulesPage";

const mockInvoke = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));

const rule: Rule = {
  pattern: "DOMAIN-SUFFIX",
  value: "example.com",
  action: { type: "DIRECT" },
  name: "Example",
  priority: 10,
  enabled: true,
  comment: "",
};

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockImplementation((command: string) => {
    if (command === "list_rule_files") return Promise.resolve(["rules.yaml"]);
    if (command === "get_rules") return Promise.resolve([]);
    return Promise.resolve(null);
  });
});

describe("RulesPage", () => {
  it("shows empty state when the selected file has no rules", async () => {
    render(<RulesPage />);
    expect(await screen.findByText("No rules configured")).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith("get_rules", { filename: "custom.yaml" });
  });

  it("shows add rule button", async () => {
    render(<RulesPage />);
    expect(await screen.findByText("+ Add Rule")).toBeInTheDocument();
  });

  it("updates a rule through the generated edit contract instead of appending", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "list_rule_files") return Promise.resolve(["custom.yaml"]);
      if (command === "get_rules") return Promise.resolve([rule]);
      return Promise.resolve(null);
    });
    const user = userEvent.setup();
    render(<RulesPage />);

    await user.click(await screen.findByRole("button", { name: "Disable rule" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("save_rule", {
        rule: { ...rule, enabled: false },
        filename: "custom.yaml",
        originalRule: rule,
      });
    });
  });
});
