import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { AiPage } from "../pages/AiPage";

describe("AiPage", () => {
  it("renders without crashing", () => {
    render(<AiPage onError={() => {}} />);
    expect(screen.getByText("Alerts")).toBeInTheDocument();
  });

  it("shows Vision Screenshot Analyzer section", () => {
    render(<AiPage onError={() => {}} />);
    expect(screen.getByText("Vision Screenshot Analyzer")).toBeInTheDocument();
  });

  it("shows AI Token Usage section", () => {
    render(<AiPage onError={() => {}} />);
    expect(screen.getByText("AI Token Usage")).toBeInTheDocument();
  });

  it("shows Scaffold Generator section", () => {
    render(<AiPage onError={() => {}} />);
    expect(screen.getByText("Scaffold Generator")).toBeInTheDocument();
  });

  it("shows Docker Deployment section", () => {
    render(<AiPage onError={() => {}} />);
    expect(screen.getByText("Docker Deployment")).toBeInTheDocument();
  });
});
