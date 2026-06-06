import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { DeployPage } from "../components/deploy/DeployPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_last_deployment") return Promise.resolve(null);
    if (cmd === "generate_deployment_bundle") return Promise.resolve(null);
    if (cmd === "write_deployment_bundle") return Promise.resolve(null);
    if (cmd === "git_init_deployment") return Promise.resolve(null);
    return Promise.resolve(null);
  }),
}));

describe("DeployPage", () => {
  it("renders deploy page", async () => {
    render(<DeployPage />);
    expect(await screen.findByText("Deploy")).toBeInTheDocument();
  });
});
