import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useNetwork } from "../../hooks/useNetwork";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => mockInvoke(...args) }));

describe("useNetwork", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads network info on mount", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_network_info") return Promise.resolve({ lan_ip: "192.168.1.100", interface: "en0" });
      if (cmd === "is_pf_enabled") return Promise.resolve(false);
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useNetwork());

    await waitFor(() => {
      expect(result.current.networkInfo).toEqual({ lan_ip: "192.168.1.100", interface: "en0" });
    });
    expect(result.current.pfEnabled).toBe(false);
  });

  it("detects pf enabled state", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_network_info") return Promise.resolve({ lan_ip: "10.0.0.1", interface: "en0" });
      if (cmd === "is_pf_enabled") return Promise.resolve(true);
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useNetwork());

    await waitFor(() => {
      expect(result.current.pfEnabled).toBe(true);
    });
  });

  it("enablePf calls setup_pf and sets pfEnabled", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_network_info") return Promise.resolve({ lan_ip: "10.0.0.1", interface: "en0" });
      if (cmd === "is_pf_enabled") return Promise.resolve(false);
      if (cmd === "setup_pf") return Promise.resolve("pf enabled");
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useNetwork());
    const onError = vi.fn();

    await waitFor(() => expect(result.current.networkInfo).not.toBeNull());

    await act(async () => {
      await result.current.enablePf(onError);
    });

    expect(result.current.pfEnabled).toBe(true);
    expect(onError).toHaveBeenCalledWith("");
  });

  it("disablePf calls teardown_pf and clears pfEnabled", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_network_info") return Promise.resolve({ lan_ip: "10.0.0.1", interface: "en0" });
      if (cmd === "is_pf_enabled") return Promise.resolve(true);
      if (cmd === "teardown_pf") return Promise.resolve();
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useNetwork());
    const onError = vi.fn();

    await waitFor(() => expect(result.current.pfEnabled).toBe(true));

    await act(async () => {
      await result.current.disablePf(onError);
    });

    expect(result.current.pfEnabled).toBe(false);
  });

  it("reports error when enablePf fails", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_network_info") return Promise.resolve({ lan_ip: "10.0.0.1", interface: "en0" });
      if (cmd === "is_pf_enabled") return Promise.resolve(false);
      if (cmd === "setup_pf") return Promise.reject(new Error("permission denied"));
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useNetwork());
    const onError = vi.fn();

    await waitFor(() => expect(result.current.networkInfo).not.toBeNull());

    await act(async () => {
      await result.current.enablePf(onError);
    });

    expect(result.current.pfEnabled).toBe(false);
    expect(onError).toHaveBeenCalledWith(expect.stringContaining("permission denied"));
  });

  it("skips setup_pf if already enabled", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_network_info") return Promise.resolve({ lan_ip: "10.0.0.1", interface: "en0" });
      if (cmd === "is_pf_enabled") return Promise.resolve(true);
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useNetwork());
    const onError = vi.fn();

    await waitFor(() => expect(result.current.pfEnabled).toBe(true));

    await act(async () => {
      await result.current.enablePf(onError);
    });

    // Should NOT have called setup_pf
    expect(mockInvoke).not.toHaveBeenCalledWith("setup_pf");
    expect(result.current.pfEnabled).toBe(true);
  });
});
