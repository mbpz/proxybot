import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useProxy } from "../../hooks/useProxy";

// Mock Tauri invoke
const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => mockInvoke(...args) }));

// Mock Tauri listen — store callbacks for manual triggering
type EventCallback = (event: { payload: unknown }) => void;
const listeners = new Map<string, EventCallback[]>();
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((event: string, cb: EventCallback) => {
    if (!listeners.has(event)) listeners.set(event, []);
    listeners.get(event)!.push(cb);
    return Promise.resolve(() => {
      const cbs = listeners.get(event);
      if (cbs) {
        const idx = cbs.indexOf(cb);
        if (idx !== -1) cbs.splice(idx, 1);
      }
    });
  }),
}));

function emitEvent(event: string, payload: unknown) {
  const cbs = listeners.get(event) || [];
  for (const cb of cbs) cb({ payload });
}

describe("useProxy", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listeners.clear();
    mockInvoke.mockResolvedValue([]);
  });

  it("initializes with default state", async () => {
    const { result } = renderHook(() => useProxy());

    expect(result.current.running).toBe(false);
    expect(result.current.requests).toEqual([]);
    expect(result.current.error).toBe("");
  });

  it("loads DNS log on mount", async () => {
    const dnsEntries = [
      { domain: "example.com", timestamp_ms: 1000 },
      { domain: "test.com", timestamp_ms: 2000 },
    ];
    mockInvoke.mockResolvedValue(dnsEntries);

    const { result } = renderHook(() => useProxy());

    await waitFor(() => {
      expect(result.current.dnsQueries).toEqual(dnsEntries);
    });
    expect(mockInvoke).toHaveBeenCalledWith("get_dns_log");
  });

  it("adds intercepted requests via events", async () => {
    const { result } = renderHook(() => useProxy());

    const req = { id: "1", host: "example.com", method: "GET", path: "/" };
    act(() => {
      emitEvent("intercepted-request", req);
    });

    expect(result.current.requests).toHaveLength(1);
    expect(result.current.requests[0]).toEqual(req);
  });

  it("prepends new requests (newest first)", async () => {
    const { result } = renderHook(() => useProxy());

    act(() => {
      emitEvent("intercepted-request", { id: "1", host: "a.com", method: "GET", path: "/" });
      emitEvent("intercepted-request", { id: "2", host: "b.com", method: "POST", path: "/" });
    });

    expect(result.current.requests).toHaveLength(2);
    expect(result.current.requests[0].id).toBe("2");
    expect(result.current.requests[1].id).toBe("1");
  });

  it("caps requests at 100", async () => {
    const { result } = renderHook(() => useProxy());

    act(() => {
      for (let i = 0; i < 110; i++) {
        emitEvent("intercepted-request", { id: String(i), host: "example.com", method: "GET", path: "/" });
      }
    });

    expect(result.current.requests).toHaveLength(100);
    // Newest first — id 109 should be first
    expect(result.current.requests[0].id).toBe("109");
  });

  it("adds DNS queries via events", async () => {
    mockInvoke.mockResolvedValue([]);
    const { result } = renderHook(() => useProxy());

    act(() => {
      emitEvent("dns-query", { domain: "api.example.com", timestamp_ms: 1000 });
    });

    expect(result.current.dnsQueries).toHaveLength(1);
    expect(result.current.dnsQueries[0].domain).toBe("api.example.com");
  });

  it("sets running to true after startProxy", async () => {
    mockInvoke.mockResolvedValue("Proxy started");
    const { result } = renderHook(() => useProxy());

    await act(async () => {
      await result.current.startProxy();
    });

    expect(result.current.running).toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith("start_proxy");
  });

  it("sets error when startProxy fails", async () => {
    mockInvoke.mockRejectedValue(new Error("port in use"));
    const { result } = renderHook(() => useProxy());

    await act(async () => {
      await result.current.startProxy();
    });

    expect(result.current.running).toBe(false);
    expect(result.current.error).toContain("port in use");
  });

  it("can clear error via setError", async () => {
    mockInvoke.mockRejectedValue(new Error("fail"));
    const { result } = renderHook(() => useProxy());

    await act(async () => {
      await result.current.startProxy();
    });
    expect(result.current.error).not.toBe("");

    act(() => {
      result.current.setError("");
    });
    expect(result.current.error).toBe("");
  });

  it("ignores invalid event payloads", async () => {
    const { result } = renderHook(() => useProxy());

    act(() => {
      emitEvent("intercepted-request", null);
      emitEvent("intercepted-request", { notId: true });
      emitEvent("intercepted-request", { id: "1", host: "valid.com", method: "GET", path: "/" });
    });

    expect(result.current.requests).toHaveLength(1);
  });
});
