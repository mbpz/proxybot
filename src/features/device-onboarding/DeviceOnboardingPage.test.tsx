import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { BrowserMockAdapter } from "../../desktop/testing";
import type { DeviceOnboarding } from "../../generated/desktop-contract";
import { CaptureSessionProvider } from "../capture-session/CaptureSession";
import { DeviceOnboardingPage } from "./DeviceOnboardingPage";

const IOS_ONBOARDING: DeviceOnboarding = {
  platform: "ios",
  interface: "en0",
  lan_ip: "192.168.1.40",
  proxy_port: 8088,
  server_url: "http://192.168.1.40:19876",
  setup_url: "http://192.168.1.40:19876/ca.crt",
  ca_url: "http://192.168.1.40:19876/ca.crt",
  qr_svg: "<svg><script>alert('unsafe')</script><rect width=\"10\" height=\"10\" /></svg>",
};

function renderPage(adapter: BrowserMockAdapter) {
  return render(
    <MemoryRouter>
      <CaptureSessionProvider contract={adapter.contract}>
        <DeviceOnboardingPage contract={adapter.contract} />
      </CaptureSessionProvider>
    </MemoryRouter>,
  );
}

describe("DeviceOnboardingPage", () => {
  it("owns capture and device preparation as separate lifecycle steps", async () => {
    let captureRunning = false;
    const adapter = new BrowserMockAdapter({
      get_proxy_status: () => captureRunning,
      start_proxy: () => {
        captureRunning = true;
        return "Proxy listening";
      },
      prepare_device_onboarding: ({ platform }) => ({ ...IOS_ONBOARDING, platform }),
      stop_device_onboarding: () => undefined,
    });
    const user = userEvent.setup();
    renderPage(adapter);

    const startCapture = await screen.findByRole("button", { name: "Start Capture" });
    await waitFor(() => expect(startCapture).toBeEnabled());
    await user.click(startCapture);
    expect(await screen.findByText("Capture is running")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Prepare iOS Setup" }));
    expect(await screen.findByText("192.168.1.40")).toBeInTheDocument();
    expect(screen.getByText("8088")).toBeInTheDocument();
    expect(screen.getByTestId("device-onboarding-qr").innerHTML).not.toContain("script");
    expect(adapter.calls).toContainEqual({
      command: "prepare_device_onboarding",
      args: { platform: "ios" },
    });

    await user.click(screen.getByRole("button", { name: "Stop Setup Server" }));
    expect(
      await screen.findByText(/Prepare this Mac to reveal the exact proxy address/),
    ).toBeInTheDocument();
    expect(adapter.calls).toContainEqual({ command: "stop_device_onboarding", args: {} });
  });

  it("prepares the Android guide without claiming universal CA trust", async () => {
    const adapter = new BrowserMockAdapter({
      get_proxy_status: () => true,
      prepare_device_onboarding: ({ platform }) => ({
        ...IOS_ONBOARDING,
        platform,
        setup_url: "http://192.168.1.40:19876/android-setup",
      }),
    });
    const user = userEvent.setup();
    renderPage(adapter);

    await user.click(await screen.findByRole("tab", { name: "Android" }));
    await user.click(screen.getByRole("button", { name: "Prepare Android Setup" }));

    expect(
      await screen.findByText(/User-installed CAs are not trusted by every Android app/),
    ).toBeInTheDocument();
    expect(adapter.calls).toContainEqual({
      command: "prepare_device_onboarding",
      args: { platform: "android" },
    });
  });

  it("keeps preparation failures visible and retryable", async () => {
    const adapter = new BrowserMockAdapter({
      get_proxy_status: () => false,
      prepare_device_onboarding: () => {
        throw new Error("No active LAN interface found");
      },
    });
    const user = userEvent.setup();
    renderPage(adapter);

    await user.click(await screen.findByRole("button", { name: "Prepare iOS Setup" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("No active LAN interface found");
    expect(screen.getByRole("button", { name: "Retry" })).toBeEnabled();
  });
});
