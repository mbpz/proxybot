import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Layout } from "./components/layout/Layout";
import { TrafficPage } from "./components/traffic/TrafficPage";
import { RulesPage } from "./components/rules/RulesPage";
import { CertsPage } from "./components/certs/CertsPage";
import { GraphPage } from "./components/graph/GraphPage";
import { ComposerPage } from "./components/composer/ComposerPage";
import "./index.css";

// Placeholder pages for now - will implement in later phases
function PlaceholderPage({ name }: { name: string }) {
  return (
    <div className="p-8">
      <h1 className="text-2xl font-bold">{name}</h1>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<TrafficPage />} />
          <Route path="rules" element={<RulesPage />} />
          <Route path="certs" element={<CertsPage />} />
          <Route path="devices" element={<PlaceholderPage name="Devices" />} />
          <Route path="dns" element={<PlaceholderPage name="DNS" />} />
          <Route path="alerts" element={<PlaceholderPage name="Alerts" />} />
          <Route path="replay" element={<PlaceholderPage name="Replay" />} />
          <Route path="composer" element={<ComposerPage />} />
          <Route path="graph" element={<GraphPage />} />
          <Route path="gen" element={<PlaceholderPage name="Gen" />} />
        </Route>
      </Routes>
    </BrowserRouter>
  </React.StrictMode>
);
