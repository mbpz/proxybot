import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DnsPage } from "../pages/DnsPage";

describe("DnsPage", () => {
  it("shows empty state when no queries", () => {
    render(<DnsPage dnsQueries={[]} />);
    expect(screen.getByText("No DNS queries")).toBeInTheDocument();
  });

  it("renders DNS query table with entries", () => {
    const queries = [
      { domain: "api.weixin.qq.com", timestamp_ms: 1713000000000, app_name: "WeChat" },
      { domain: "douyin.com", timestamp_ms: 1713000001000, app_name: "Douyin" },
    ];
    render(<DnsPage dnsQueries={queries} />);
    expect(screen.getByText("api.weixin.qq.com")).toBeInTheDocument();
    expect(screen.getByText("douyin.com")).toBeInTheDocument();
    expect(screen.getByText("WeChat")).toBeInTheDocument();
    expect(screen.getByText("Douyin")).toBeInTheDocument();
  });

  it("shows entry count in header", () => {
    const queries = [
      { domain: "example.com", timestamp_ms: 1713000000000 },
    ];
    render(<DnsPage dnsQueries={queries} />);
    expect(screen.getByText("1 entries")).toBeInTheDocument();
  });

  it("renders without app badge when app_name is missing", () => {
    const queries = [
      { domain: "example.com", timestamp_ms: 1713000000000 },
    ];
    render(<DnsPage dnsQueries={queries} />);
    expect(screen.getByText("example.com")).toBeInTheDocument();
  });
});
