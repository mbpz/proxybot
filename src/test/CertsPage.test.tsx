import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { CertsPage } from "../components/certs/CertsPage";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === "get_ca_metadata") return Promise.resolve(null);
    if (cmd === "is_ca_server_running") return Promise.resolve(false);
    if (cmd === "get_ca_cert_pem") return Promise.resolve("");
    if (cmd === "get_network_info") return Promise.resolve({ lan_ip: "127.0.0.1" });
    return Promise.resolve(null);
  }),
}));

describe("CertsPage", () => {
  it("renders certificates page", async () => {
    render(<CertsPage />);
    expect(await screen.findByText("Certificates")).toBeInTheDocument();
    expect(await screen.findByText("Root CA Certificate")).toBeInTheDocument();
  });
});
