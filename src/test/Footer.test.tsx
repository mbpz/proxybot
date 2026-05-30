import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Footer } from "../components/layout/Footer";

const defaultProps = {
  networkInfo: { lan_ip: "192.168.1.100", interface: "en0" },
  pfEnabled: false,
  pfLoading: false,
  tunEnabled: false,
  tunLoading: false,
  dashboardRunning: false,
  dashboardUrl: "",
  onEnablePf: () => {},
  onDisablePf: () => {},
  onEnableTun: () => {},
  onDisableTun: () => {},
  onToggleDashboard: () => {},
};

describe("Footer", () => {
  it("shows LAN IP", () => {
    render(<Footer {...defaultProps} />);
    expect(screen.getByText("192.168.1.100")).toBeInTheDocument();
  });

  it("shows pf disabled state", () => {
    render(<Footer {...defaultProps} />);
    const disabled = screen.getAllByText("Disabled");
    expect(disabled.length).toBe(2); // pf and TUN both disabled
    expect(screen.getByText("Enable pf")).toBeInTheDocument();
  });

  it("shows pf enabled state", () => {
    render(<Footer {...defaultProps} pfEnabled={true} />);
    expect(screen.getByText("Enabled")).toBeInTheDocument();
    expect(screen.getByText("Disable pf")).toBeInTheDocument();
  });

  it("shows TUN mode button when not enabled", () => {
    render(<Footer {...defaultProps} />);
    expect(screen.getByText("TUN Mode")).toBeInTheDocument();
  });

  it("hides TUN mode button when enabled", () => {
    render(<Footer {...defaultProps} tunEnabled={true} />);
    expect(screen.queryByText("TUN Mode")).not.toBeInTheDocument();
  });

  it("shows loading state for pf", () => {
    render(<Footer {...defaultProps} pfLoading={true} />);
    expect(screen.getByText("...")).toBeInTheDocument();
  });
});
