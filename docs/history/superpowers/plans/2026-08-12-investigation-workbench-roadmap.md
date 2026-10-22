# ProxyBot Investigation Workbench Implementation Roadmap

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this roadmap one batch at a time. Each batch has its own task-level plan and review gate.

**Goal:** Converge ProxyBot on the approved macOS investigation-workbench product while keeping every delivery independently testable and revertible.

**Architecture:** The roadmap follows one vertical dependency chain: stabilize the current baseline, converge desktop communication, introduce persistent CaptureSession identity, deliver complete evidence inspection, complete modify/reproduce/share, add evidence-backed inference, then enforce capability and release discipline. A batch may start only after its predecessor's required gate is committed and green.

**Tech Stack:** Rust, Tauri 2, SQLite/rusqlite, React 19, TypeScript 5.8, Vitest, Playwright, pnpm 10.33.0, GitHub Actions, macOS signing/notarization tools

## Global Constraints

- Product positioning is “macOS MITM investigation workbench for developers debugging iOS and Android test devices.”
- Primary journey is `Setup -> Capture Session -> Investigate -> Modify/Reproduce -> Redacted Share`.
- Preserve observed evidence; Activities, Findings, and Relationships are versioned rebuildable projections.
- Tauri/React remains the desktop Implementation; do not introduce AppKit, SwiftUI, a packet five-tuple model, PCAP schema, or a privileged Helper.
- Production React code communicates with the desktop only through generated `DesktopContract`.
- `package.json` remains the Product Version and pnpm version authority.
- Every shell command in this repository starts with `rtk`.
- Every task follows red-green-refactor, runs its scoped gate, and ends with an independent commit.
- Preserve unrelated working-tree changes and stage exact paths only.

---

## Ordered plans

1. [Batch 0 — Baseline Stabilization](2026-08-12-batch-0-baseline-stabilization.md)
2. [Batch 1 — Desktop Contract Convergence](2026-08-12-batch-1-desktop-contract-convergence.md)
3. [Batch 2 — Persistent CaptureSession](2026-08-12-batch-2-persistent-capture-session.md)
4. [Batch 3 — Evidence Investigation Workspace](2026-08-12-batch-3-evidence-investigation-workspace.md)
5. [Batch 4 — Modify, Reproduce, and Redacted Share](2026-08-12-batch-4-modify-reproduce-share.md)
6. [Batch 5 — Activity and Context Projections](2026-08-12-batch-5-activity-context-projections.md)
7. [Batch 6 — Capability and Verified Release](2026-08-12-batch-6-capability-verified-release.md)

## Cross-batch gates

| After batch | Required evidence before continuing |
| --- | --- |
| 0 | Current Alerts slice committed; hosted CI executes Rust, frontend, E2E, and bundle jobs past pnpm setup |
| 1 | No production raw `invoke`, `listen`, or `safeInvoke`; Graph and DAG payloads are distinct and validated |
| 2 | Every newly captured fact has one durable CaptureSession; stop/restart recovery is proven |
| 3 | A selected Captured Request loads complete persisted Request/Response evidence in one workspace |
| 4 | Edit/replay lineage is preserved and share is redacted by default |
| 5 | Activity/Finding/Relationship claims link to source evidence and uncertain records remain ungrouped |
| 6 | Only re-downloaded, independently verified assets can become a public Verified Release |

Each gate requires a clean scoped diff, the plan's commands, and a reviewer decision. Do not combine adjacent batches merely because their files overlap.

## Approved-spec coverage

| Design requirement | Owning plan |
| --- | --- |
| Product promise, five Product Destinations, Capability levels | Batch 6 Tasks 1–2; roadmap global constraints |
| Persistent CaptureSession lifecycle, evidence identity, and recovery | Batch 2 Tasks 1–7 |
| Observed evidence versus rebuildable Activity | Batch 2 Task 4; Batch 5 Tasks 1–2 |
| Stable Investigation Workspace, Focus Sets, Noise Control, Inspector, Context Dock | Batch 3 Tasks 3–6 |
| Single Desktop Contract, structured errors, DTO projection discipline | Batch 1 Tasks 1–6; Batch 3 Tasks 1–2 |
| Edit-and-forward, request-derived Replay/Composer, redacted share | Batch 4 Tasks 1–6 |
| Findings and Relationships replacing standalone analysis destinations | Batch 5 Tasks 3–6 |
| BuildFlavor, locked resources, release manifest, uploaded-asset and physical-device gates | Batch 6 Tasks 3–8 |

No task adds Tracexy's packet data plane, AppKit UI, fixed DNS timing rule, or privileged Helper.
