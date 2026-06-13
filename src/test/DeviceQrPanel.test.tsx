import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DeviceQrPanel } from "../components/setup/DeviceQrPanel";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const QR_SVG = '<svg><rect width="100" height="100"/></svg>';

describe("DeviceQrPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders with iOS tab selected by default", async () => {
    mockInvoke.mockResolvedValue(QR_SVG);
    render(<DeviceQrPanel />);

    expect(screen.getByRole("tab", { name: "iOS" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tab", { name: "Android" })).toHaveAttribute("aria-selected", "false");
    expect(await screen.findByText("Add Mobile Device")).toBeInTheDocument();
  });

  it("shows loading state while waiting for invoke", () => {
    mockInvoke.mockReturnValue(new Promise(() => {}));
    render(<DeviceQrPanel />);

    expect(screen.getByText("Loading...")).toBeInTheDocument();
  });

  it("renders QR SVG when invoke succeeds", async () => {
    mockInvoke.mockResolvedValue(QR_SVG);
    render(<DeviceQrPanel />);

    const qrContainer = await screen.findByTestId("device-qr-svg");
    expect(qrContainer).toBeInTheDocument();
    expect(qrContainer.innerHTML).toContain("<svg");
    expect(screen.queryByText("Loading...")).not.toBeInTheDocument();
  });

  it("shows error message when invoke fails", async () => {
    mockInvoke.mockRejectedValue(new Error("server offline"));
    render(<DeviceQrPanel />);

    expect(await screen.findByText("Error: server offline")).toBeInTheDocument();
    expect(screen.queryByText("Loading...")).not.toBeInTheDocument();
  });

  it("switches to Android tab and calls invoke with 'android'", async () => {
    mockInvoke.mockResolvedValue(QR_SVG);
    const user = userEvent.setup();
    render(<DeviceQrPanel />);

    await screen.findByTestId("device-qr-svg");
    expect(mockInvoke).toHaveBeenCalledWith("generate_device_qr", { platform: "ios" });

    await user.click(screen.getByRole("tab", { name: "Android" }));

    expect(screen.getByRole("tab", { name: "Android" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tab", { name: "iOS" })).toHaveAttribute("aria-selected", "false");
    expect(mockInvoke).toHaveBeenCalledWith("generate_device_qr", { platform: "android" });
  });

  it("shows iOS details section only when iOS tab is selected", async () => {
    mockInvoke.mockResolvedValue(QR_SVG);
    const user = userEvent.setup();
    render(<DeviceQrPanel />);

    // iOS selected — details visible
    expect(await screen.findByText("After installing the profile")).toBeInTheDocument();

    // Switch to Android — details hidden
    await user.click(screen.getByRole("tab", { name: "Android" }));
    expect(screen.queryByText("After installing the profile")).not.toBeInTheDocument();

    // Switch back to iOS — details visible again
    await user.click(screen.getByRole("tab", { name: "iOS" }));
    expect(screen.getByText("After installing the profile")).toBeInTheDocument();
  });
});
