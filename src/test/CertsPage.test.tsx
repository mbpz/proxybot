import { beforeEach, describe, it, expect, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { CertsPage } from "../components/certs/CertsPage";

function defaultInvoke(cmd: string) {
  if (cmd === "get_ca_metadata") return Promise.resolve(null);
  if (cmd === "is_cert_server_running") return Promise.resolve(false);
  if (cmd === "get_ca_cert_path") return Promise.resolve("/tmp/ca.pem");
  if (cmd === "get_network_info") return Promise.resolve({ lan_ip: "127.0.0.1" });
  if (cmd === "start_cert_server") return Promise.resolve("http://127.0.0.1:8090");
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
    render(<CertsPage />);
    expect(await screen.findByText("Certificates")).toBeInTheDocument();
    expect(await screen.findByText("Root CA Certificate")).toBeInTheDocument();
  });

  it("stops the certificate listener before clearing its running state", async () => {
    render(<CertsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Start CA Server" }));
    await screen.findByRole("button", { name: "Stop CA Server" });

    fireEvent.click(screen.getByRole("button", { name: "Stop CA Server" }));

    await waitFor(() => expect(invoke).toHaveBeenCalledWith("stop_cert_server"));
    expect(await screen.findByRole("button", { name: "Start CA Server" })).toBeInTheDocument();
  });

  it("observes an existing certificate listener after remount", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "is_cert_server_running") return Promise.resolve(true);
      return defaultInvoke(cmd);
    });

    render(<CertsPage />);

    fireEvent.click(await screen.findByRole("button", { name: "Stop CA Server" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("stop_cert_server"));
  });
});
