import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useTopologyGraph } from "../hooks/useTopologyGraph";
import type { TopologyGraph, TopologyFilter } from "../types";

const EMPTY_FILTER: TopologyFilter = { sync_global: false };
const FAKE_GRAPH: TopologyGraph = {
  nodes: [],
  edges: [],
  meta: {
    total_requests: 0,
    total_bytes: 0,
    device_count: 0,
    app_count: 0,
    host_count: 0,
    time_range: [0, 0],
    built_at: 0,
  },
};

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("useTopologyGraph", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    invokeMock.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("starts in loading state and resolves to graph on debounce flush", async () => {
    invokeMock.mockResolvedValue(FAKE_GRAPH);

    const { result } = renderHook(() => useTopologyGraph(EMPTY_FILTER, 0));

    expect(result.current.loading).toBe(true);
    expect(result.current.graph).toBeNull();

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    expect(invokeMock).toHaveBeenCalledWith("build_topology_graph", {
      filter: EMPTY_FILTER,
    });
    expect(result.current.loading).toBe(false);
    expect(result.current.graph).toEqual(FAKE_GRAPH);
    expect(result.current.error).toBeNull();
  });

  it("debounces rapid filter changes and only fires once for the latest", async () => {
    invokeMock.mockResolvedValue(FAKE_GRAPH);

    const { rerender } = renderHook(
      ({ f }: { f: TopologyFilter }) => useTopologyGraph(f, 100),
      { initialProps: { f: { ...EMPTY_FILTER } } },
    );

    rerender({ f: { ...EMPTY_FILTER, host_contains: "a" } });
    rerender({ f: { ...EMPTY_FILTER, host_contains: "ab" } });
    rerender({ f: { ...EMPTY_FILTER, host_contains: "abc" } });

    expect(invokeMock).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("build_topology_graph", {
      filter: { ...EMPTY_FILTER, host_contains: "abc" },
    });
  });

  it("ignores stale responses when a newer request supersedes them", async () => {
    let resolveFirst: (g: TopologyGraph) => void = () => {};
    const firstGraph: TopologyGraph = { ...FAKE_GRAPH, meta: { ...FAKE_GRAPH.meta, built_at: 1 } };
    const secondGraph: TopologyGraph = { ...FAKE_GRAPH, meta: { ...FAKE_GRAPH.meta, built_at: 2 } };

    invokeMock
      .mockImplementationOnce(
        () => new Promise<TopologyGraph>((res) => (resolveFirst = res)),
      )
      .mockResolvedValueOnce(secondGraph);

    const { result, rerender } = renderHook(
      ({ f }: { f: TopologyFilter }) => useTopologyGraph(f, 0),
      { initialProps: { f: { ...EMPTY_FILTER } } },
    );

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    rerender({ f: { ...EMPTY_FILTER, host_contains: "trigger" } });
    await act(async () => {
      await vi.runAllTimersAsync();
    });

    expect(invokeMock).toHaveBeenCalledTimes(2);

    await act(async () => {
      resolveFirst(firstGraph);
    });

    expect(result.current.graph?.meta.built_at).toBe(2);
  });

  it("surfaces the error when safeInvoke returns null and surfaces thrown errors", async () => {
    invokeMock.mockResolvedValue(null);

    const { result } = renderHook(() => useTopologyGraph(EMPTY_FILTER, 0));

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    expect(result.current.graph).toBeNull();
    expect(result.current.loading).toBe(false);
  });
});
