import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { BrowserMockAdapter } from "../../desktop/testing";
import { CaptureSessionBar, CaptureSessionProvider } from "./CaptureSession";

function renderSession(adapter: BrowserMockAdapter) {
  return render(
    <MemoryRouter>
      <CaptureSessionProvider contract={adapter.contract}>
        <CaptureSessionBar />
      </CaptureSessionProvider>
    </MemoryRouter>,
  );
}

describe("CaptureSession", () => {
  it("loads the backend state before offering capture", async () => {
    const adapter = new BrowserMockAdapter({
      get_proxy_status: () => false,
    });

    renderSession(adapter);

    expect(await screen.findByText("Capture stopped")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start Capture" })).toBeEnabled();
    expect(adapter.calls).toContainEqual({ command: "get_proxy_status", args: {} });
  });

  it("starts and stops one Capture Session", async () => {
    let running = false;
    const adapter = new BrowserMockAdapter({
      get_proxy_status: () => running,
      start_proxy: () => {
        running = true;
        return "Proxy listening";
      },
      stop_proxy: () => {
        running = false;
        return "Proxy stopped";
      },
    });
    const user = userEvent.setup();
    renderSession(adapter);

    await user.click(await screen.findByRole("button", { name: "Start Capture" }));
    expect(await screen.findByText("Capturing")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Stop Capture" }));
    expect(await screen.findByText("Capture stopped")).toBeInTheDocument();
    expect(adapter.calls.map(({ command }) => command)).toEqual([
      "get_proxy_status",
      "start_proxy",
      "stop_proxy",
    ]);
  });

  it("reconciles status and exposes recovery after a lifecycle failure", async () => {
    const adapter = new BrowserMockAdapter({
      get_proxy_status: () => false,
      start_proxy: () => {
        throw new Error("port 8088 is already in use");
      },
    });
    const user = userEvent.setup();
    renderSession(adapter);

    await user.click(await screen.findByRole("button", { name: "Start Capture" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("port 8088 is already in use");
    expect(screen.getByText("Capture stopped")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Retry" })).toBeEnabled();

    await user.click(screen.getByRole("button", { name: "Dismiss capture error" }));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("tracks lifecycle changes made through the tray Adapter", async () => {
    const adapter = new BrowserMockAdapter({
      get_proxy_status: () => false,
    });
    renderSession(adapter);
    await screen.findByText("Capture stopped");

    act(() => adapter.emit("capture-session:changed", true));

    expect(await screen.findByText("Capturing")).toBeInTheDocument();
  });

  it("reports invalid lifecycle events through the contract", async () => {
    const adapter = new BrowserMockAdapter({
      get_proxy_status: () => false,
    });
    renderSession(adapter);
    await screen.findByText("Capture stopped");

    act(() => adapter.emit("capture-session:changed", "yes" as unknown as boolean));

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("must be a boolean");
    });
  });
});
