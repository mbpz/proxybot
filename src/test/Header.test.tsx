import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Header } from "../components/layout/Header";

describe("Header", () => {
  it("shows stopped state when not running", () => {
    render(<Header running={false} caMetadata={null} onStart={() => {}} onDownloadCa={() => {}} />);
    expect(screen.getByText("Stopped")).toBeInTheDocument();
    expect(screen.getByText("Start")).toBeInTheDocument();
  });

  it("shows running state when running", () => {
    render(<Header running={true} caMetadata={null} onStart={() => {}} onDownloadCa={() => {}} />);
    expect(screen.getByText("Proxy running on :8080")).toBeInTheDocument();
    expect(screen.getByText("Running")).toBeInTheDocument();
  });

  it("shows CA date when metadata is present", () => {
    const caMetadata = { created_at: 1713000000, serial: "abc123" };
    render(<Header running={false} caMetadata={caMetadata} onStart={() => {}} onDownloadCa={() => {}} />);
    expect(screen.getByText(/CA:/)).toBeInTheDocument();
  });

  it("disables start button when running", () => {
    render(<Header running={true} caMetadata={null} onStart={() => {}} onDownloadCa={() => {}} />);
    expect(screen.getByText("Running")).toBeDisabled();
  });
});
