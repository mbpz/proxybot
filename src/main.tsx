import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Layout } from "./components/layout/Layout";
import "./index.css";

// Placeholder pages for now - will implement in later tasks
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
          <Route index element={<div className="p-8"><h1 className="text-2xl">Traffic Page - Coming Soon</h1></div>} />
          <Route path="rules" element={<PlaceholderPage name="Rules" />} />
          <Route path="certs" element={<PlaceholderPage name="Certs" />} />
          <Route path="devices" element={<PlaceholderPage name="Devices" />} />
          <Route path="dns" element={<PlaceholderPage name="DNS" />} />
          <Route path="alerts" element={<PlaceholderPage name="Alerts" />} />
          <Route path="replay" element={<PlaceholderPage name="Replay" />} />
          <Route path="graph" element={<PlaceholderPage name="Graph" />} />
          <Route path="gen" element={<PlaceholderPage name="Gen" />} />
        </Route>
      </Routes>
    </BrowserRouter>
  </React.StrictMode>
);
