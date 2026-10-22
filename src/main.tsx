import React, { lazy } from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { Layout } from "./components/layout/Layout";
import { TrafficPage } from "./components/traffic/TrafficPage";
import { RulesPage } from "./components/rules/RulesPage";
import { CertsPage } from "./components/certs/CertsPage";
import { DevicesPage } from "./components/devices/DevicesPage";
import { DnsPage } from "./components/dns/DnsPage";
import { ReplayPage } from "./components/replay/ReplayPage";
import { ComposerPage } from "./components/composer/ComposerPage";
import { AlertsPage } from "./components/alerts/AlertsPage";
import { SettingsPage } from "./components/settings/SettingsPage";
import { DeviceOnboardingPage } from "./features/device-onboarding/DeviceOnboardingPage";
import { CaptureWorkspace } from "./features/capture-session/CaptureWorkspace";
import { ReplayWorkspace } from "./features/replay-workspace/ReplayWorkspace";
import { LabsProvider } from "./features/labs/LabsContext";
import { LabsGate } from "./features/labs/LabsGate";
import "./index.css";

// Labs surfaces are code-split so their heavy dependencies (mermaid,
// vis-network, recharts, highlight.js) stay out of the initial shell bundle.
const GraphPage = lazy(() =>
  import("./components/graph/GraphPage").then((module) => ({ default: module.GraphPage })),
);
const TopologyPage = lazy(() =>
  import("./components/topology/TopologyPage").then((module) => ({ default: module.TopologyPage })),
);
const AiPage = lazy(() =>
  import("./components/ai/AiPage").then((module) => ({ default: module.AiPage })),
);
const GenPage = lazy(() =>
  import("./components/gen/GenPage").then((module) => ({ default: module.GenPage })),
);
const DeployPage = lazy(() =>
  import("./components/deploy/DeployPage").then((module) => ({ default: module.DeployPage })),
);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <BrowserRouter>
      <LabsProvider>
        <Routes>
          <Route path="/" element={<Layout />}>
            <Route element={<CaptureWorkspace />}>
              <Route index element={<TrafficPage />} />
              <Route path="dns" element={<DnsPage />} />
              <Route path="alerts" element={<AlertsPage />} />
              <Route path="rules" element={<RulesPage />} />
              <Route
                path="graph"
                element={
                  <LabsGate>
                    <GraphPage />
                  </LabsGate>
                }
              />
              <Route
                path="topology"
                element={
                  <LabsGate>
                    <TopologyPage />
                  </LabsGate>
                }
              />
            </Route>
            <Route path="setup" element={<DeviceOnboardingPage />} />
            <Route path="certs" element={<CertsPage />} />
            <Route path="devices" element={<DevicesPage />} />
            <Route element={<ReplayWorkspace />}>
              <Route path="replay" element={<ReplayPage />} />
              <Route path="composer" element={<ComposerPage />} />
            </Route>
            <Route
              path="gen"
              element={
                <LabsGate>
                  <GenPage />
                </LabsGate>
              }
            />
            <Route
              path="deploy"
              element={
                <LabsGate>
                  <DeployPage />
                </LabsGate>
              }
            />
            <Route
              path="ai"
              element={
                <LabsGate>
                  <AiPage />
                </LabsGate>
              }
            />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </LabsProvider>
    </BrowserRouter>
  </React.StrictMode>,
);
