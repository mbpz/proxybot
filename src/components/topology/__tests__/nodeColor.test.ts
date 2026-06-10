import { describe, it, expect } from "vitest";
import { nodeBackgroundColor, nodeBorderColor, nodeSize } from "../nodeColor";
import { TopologyNode } from "../types";

function makeNode(overrides: Partial<TopologyNode> = {}): TopologyNode {
  return {
    id: "test",
    kind: "host",
    label: "test.com",
    app_tag: null,
    device_id: null,
    request_count: 1,
    total_bytes: 0,
    avg_latency_ms: 0,
    error_count: 0,
    error_rate: 0,
    last_seen: 0,
    ...overrides,
  };
}

describe("nodeColor", () => {
  it("returns device color for device kind", () => {
    expect(nodeBackgroundColor(makeNode({ kind: "device" }))).toContain("0,212,255");
  });

  it("returns error border for high error rate", () => {
    const node = makeNode({ error_rate: 0.15 });
    expect(nodeBorderColor(node)).toBe("#ff4d4d");
  });

  it("returns normal border for low error rate", () => {
    const node = makeNode({ kind: "host", error_rate: 0.05 });
    expect(nodeBorderColor(node)).toBe("#1e1e2e");
  });

  it("scales size by log of request count", () => {
    const small = nodeSize(makeNode({ request_count: 1 }));
    const large = nodeSize(makeNode({ request_count: 1000 }));
    expect(large).toBeGreaterThan(small);
  });
});
