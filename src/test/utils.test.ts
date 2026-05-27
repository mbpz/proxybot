import { describe, it, expect } from "vitest";
import { formatTimestamp, formatSize, formatBytes, formatBody, appBadgeClass } from "../utils";

describe("formatTimestamp", () => {
  it("formats unix seconds with milliseconds", () => {
    const result = formatTimestamp("1713000000.123");
    // toLocaleTimeString may include AM/PM depending on locale
    expect(result).toContain(".123");
  });

  it("handles missing milliseconds", () => {
    const result = formatTimestamp("1713000000");
    expect(result).toContain(".000");
  });

  it("handles invalid input without throwing", () => {
    expect(formatTimestamp("not-a-timestamp")).toBe("not-a-timestamp");
    expect(formatTimestamp("")).toBe("");
  });
});

describe("formatSize", () => {
  it("formats bytes", () => {
    expect(formatSize(500)).toBe("500B");
  });

  it("formats kilobytes", () => {
    expect(formatSize(1536)).toBe("1.5KB");
  });

  it("formats megabytes", () => {
    expect(formatSize(2 * 1024 * 1024)).toBe("2.0MB");
  });
});

describe("formatBytes", () => {
  it("formats bytes", () => {
    expect(formatBytes(500)).toBe("500B");
  });

  it("formats kilobytes", () => {
    expect(formatBytes(1536)).toBe("1.5KB");
  });

  it("formats megabytes", () => {
    expect(formatBytes(2 * 1024 * 1024)).toBe("2.0MB");
  });

  it("formats gigabytes", () => {
    expect(formatBytes(3 * 1024 * 1024 * 1024)).toBe("3.0GB");
  });
});

describe("formatBody", () => {
  it("returns empty string for undefined body", () => {
    expect(formatBody(undefined, [])).toBe("");
  });

  it("returns empty string for empty body", () => {
    expect(formatBody("", [])).toBe("");
  });

  it("pretty-prints JSON body", () => {
    const body = '{"key":"value","nested":{"a":1}}';
    const headers: [string, string][] = [["content-type", "application/json"]];
    const result = formatBody(body, headers);
    expect(result).toContain('"key": "value"');
    expect(result).toContain('"a": 1');
  });

  it("returns raw body for non-JSON content-type", () => {
    const body = "<html>hello</html>";
    const headers: [string, string][] = [["content-type", "text/html"]];
    expect(formatBody(body, headers)).toBe(body);
  });

  it("returns raw body when JSON parsing fails", () => {
    const body = "{invalid json}";
    const headers: [string, string][] = [["content-type", "application/json"]];
    expect(formatBody(body, headers)).toBe(body);
  });
});

describe("appBadgeClass", () => {
  it("returns wechat badge for WeChat", () => {
    expect(appBadgeClass("WeChat")).toBe("badge-wechat");
  });

  it("returns douyin badge for Douyin", () => {
    expect(appBadgeClass("Douyin")).toBe("badge-douyin");
  });

  it("returns alipay badge for Alipay", () => {
    expect(appBadgeClass("Alipay")).toBe("badge-alipay");
  });

  it("returns unknown badge for unknown app", () => {
    expect(appBadgeClass("SomeApp")).toBe("badge-unknown");
  });

  it("returns unknown badge for undefined", () => {
    expect(appBadgeClass(undefined)).toBe("badge-unknown");
  });

  it("is case-insensitive", () => {
    expect(appBadgeClass("wechat")).toBe("badge-wechat");
    expect(appBadgeClass("DOUYIN")).toBe("badge-douyin");
  });
});
