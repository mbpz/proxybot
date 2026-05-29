import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useCa } from "../../hooks/useCa";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => mockInvoke(...args) }));

describe("useCa", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads CA metadata on mount", async () => {
    const metadata = { created_at: 1713000000, serial: "abc123" };
    mockInvoke.mockResolvedValue(metadata);

    const { result } = renderHook(() => useCa());

    await waitFor(() => {
      expect(result.current.caMetadata).toEqual(metadata);
    });
    expect(mockInvoke).toHaveBeenCalledWith("get_ca_metadata");
  });

  it("handles null CA metadata", async () => {
    mockInvoke.mockResolvedValue(null);

    const { result } = renderHook(() => useCa());

    await waitFor(() => {
      expect(result.current.caMetadata).toBeNull();
    });
  });

  it("downloadCaCert copies PEM to clipboard", async () => {
    const pem = "-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----";
    mockInvoke.mockResolvedValue(pem);

    // Mock clipboard API
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });
    // Mock alert
    vi.stubGlobal("alert", vi.fn());

    const { result } = renderHook(() => useCa());
    const onError = vi.fn();

    await act(async () => {
      await result.current.downloadCaCert(onError);
    });

    expect(mockInvoke).toHaveBeenCalledWith("get_ca_cert_pem");
    expect(writeText).toHaveBeenCalledWith(pem);
    expect(window.alert).toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });

  it("downloadCaCert reports error on failure", async () => {
    mockInvoke.mockRejectedValue(new Error("CA not initialized"));

    const { result } = renderHook(() => useCa());
    const onError = vi.fn();

    await act(async () => {
      await result.current.downloadCaCert(onError);
    });

    expect(onError).toHaveBeenCalledWith(expect.stringContaining("CA not initialized"));
  });
});
