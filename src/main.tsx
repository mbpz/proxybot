import React from "react";
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
import { GraphPage } from "./components/graph/GraphPage";
import { TopologyPage } from "./components/topology/TopologyPage";
import { AlertsPage } from "./components/alerts/AlertsPage";
import { AiPage } from "./components/ai/AiPage";
import { GenPage } from "./components/gen/GenPage";
import { DeployPage } from "./components/deploy/DeployPage";
import { SslBypassPage } from "./components/ssl-bypass/SslBypassPage";
import { SettingsPage } from "./components/settings/SettingsPage";
import { DeviceOnboardingPage } from "./features/device-onboarding/DeviceOnboardingPage";
import { CaptureWorkspace } from "./features/capture-session/CaptureWorkspace";
import { ReplayWorkspace } from "./features/replay-workspace/ReplayWorkspace";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route element={<CaptureWorkspace />}>
            <Route index element={<TrafficPage />} />
            <Route path="dns" element={<DnsPage />} />
            <Route path="alerts" element={<AlertsPage />} />
            <Route path="graph" element={<GraphPage />} />
            <Route path="topology" element={<TopologyPage />} />
          </Route>
          <Route path="setup" element={<DeviceOnboardingPage />} />
          <Route path="rules" element={<RulesPage />} />
          <Route path="certs" element={<CertsPage />} />
          <Route path="devices" element={<DevicesPage />} />
          <Route element={<ReplayWorkspace />}>
            <Route path="replay" element={<ReplayPage />} />
            <Route path="composer" element={<ComposerPage />} />
          </Route>
          <Route path="gen" element={<GenPage />} />
          <Route path="deploy" element={<DeployPage />} />
          <Route path="ai" element={<AiPage />} />
          <Route path="ssl-bypass" element={<SslBypassPage />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
  </React.StrictMode>
);
