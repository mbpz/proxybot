import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useUpdateCheck, CURRENT_VERSION } from "../hooks/useUpdateCheck";

describe("CURRENT_VERSION", () => {
  it("is on the v1.3.x line", () => {
    // Pinned to v1.3.0 per the 2026-05-14 update-icon spec + roadmap.
    // Bumping this requires updating the spec self-review notes.
    expect(CURRENT_VERSION).toBe("1.3.0");
  });
});

describe("useUpdateCheck", () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  function mockRelease(tag_name: string) {
    globalThis.fetch = (async () =>
      ({
        ok: true,
        status: 200,
        json: async () => ({ tag_name, html_url: "https://example/release" }),
      } as Response)) as typeof fetch;
  }

  it("reports hasUpdate=true when latest > current", async () => {
    mockRelease("v1.4.0");
    const { result } = renderHook(() => useUpdateCheck());
    await act(async () => {
      await result.current.checkForUpdates();
    });
    expect(result.current.hasUpdate).toBe(true);
    expect(result.current.latestVersion).toBe("1.4.0");
  });

  it("reports hasUpdate=false when latest == current", async () => {
    mockRelease(`v${CURRENT_VERSION}`);
    const { result } = renderHook(() => useUpdateCheck());
    await act(async () => {
      await result.current.checkForUpdates();
    });
    expect(result.current.hasUpdate).toBe(false);
  });

  it("reports hasUpdate=false when latest < current", async () => {
    mockRelease("v1.2.5");
    const { result } = renderHook(() => useUpdateCheck());
    await act(async () => {
      await result.current.checkForUpdates();
    });
    expect(result.current.hasUpdate).toBe(false);
  });

  it("strips the leading 'v' from tag_name", async () => {
    mockRelease("v2.0.0");
    const { result } = renderHook(() => useUpdateCheck());
    await act(async () => {
      await result.current.checkForUpdates();
    });
    expect(result.current.latestVersion).toBe("2.0.0");
  });

  it("sets error on HTTP failure but leaves hasUpdate=false", async () => {
    globalThis.fetch = (async () =>
      ({
        ok: false,
        status: 500,
        json: async () => ({}),
      } as Response)) as typeof fetch;
    const { result } = renderHook(() => useUpdateCheck());
    await act(async () => {
      await result.current.checkForUpdates();
    });
    expect(result.current.error).toMatch(/500/);
    expect(result.current.hasUpdate).toBe(false);
    expect(result.current.isLoading).toBe(false);
  });

  it("exposes openReleasePage that returns the release URL when present", async () => {
    mockRelease("v1.4.0");
    const { result } = renderHook(() => useUpdateCheck());
    await act(async () => {
      await result.current.checkForUpdates();
    });
    expect(result.current.releaseUrl).toBe("https://example/release");
  });
});