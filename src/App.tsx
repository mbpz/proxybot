import { useState, useEffect } from "react";
import { useUpdateCheck } from "./hooks/useUpdateCheck";
import { useProxy } from "./hooks/useProxy";
import { useNetwork } from "./hooks/useNetwork";
import { useCa } from "./hooks/useCa";
import { Header } from "./components/layout/Header";
import { Footer } from "./components/layout/Footer";
import { TrafficPage } from "./pages/TrafficPage";
import { DnsPage } from "./pages/DnsPage";
import { RulesPage } from "./pages/RulesPage";
import { DevicesPage } from "./pages/DevicesPage";
import { AiPage } from "./pages/AiPage";
import "./index.css";

type TopTab = "traffic" | "dns" | "rules" | "devices" | "ai";

function App() {
  const { running, requests, dnsQueries, error, startProxy, setError } = useProxy();
  const { networkInfo, pfEnabled, pfLoading, tunEnabled, tunLoading, enablePf, disablePf, enableTun } = useNetwork();
  const { caMetadata, downloadCaCert } = useCa();
  const { checkForUpdates } = useUpdateCheck();

  const [activeTopTab, setActiveTopTab] = useState<TopTab>("traffic");

  useEffect(() => {
    const timer = setTimeout(() => checkForUpdates(), 3000);
    return () => clearTimeout(timer);
  }, []);

  return (
    <div style={{ minHeight: "100vh", background: "var(--bg-primary)" }}>
      <Header
        running={running}
        caMetadata={caMetadata}
        onStart={startProxy}
        onDownloadCa={() => downloadCaCert(setError)}
      />

      {/* Top tab bar */}
      <div className="tabs" style={{ padding: "0 var(--space-4)", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border)" }}>
        {(["traffic", "dns", "rules", "devices", "ai"] as const).map((tab) => (
          <button key={tab} className={`tab ${activeTopTab === tab ? "active" : ""}`} onClick={() => setActiveTopTab(tab)}>
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      {/* Error banner */}
      {error && (
        <div style={{ padding: "var(--space-4) var(--space-4) 0" }}>
          <div className="error-banner"><span className="error-banner-message">{error}</span></div>
        </div>
      )}

      {/* Main content */}
      <div style={{ padding: "var(--space-4)", display: "flex", gap: "var(--space-4)", flexDirection: "column", paddingBottom: 80 }}>
        {activeTopTab === "traffic" && <TrafficPage requests={requests} />}
        {activeTopTab === "dns" && <DnsPage dnsQueries={dnsQueries} />}
        {activeTopTab === "rules" && <RulesPage />}
        {activeTopTab === "devices" && <DevicesPage networkInfo={networkInfo} />}
        {activeTopTab === "ai" && <AiPage />}
      </div>

      <Footer
        networkInfo={networkInfo}
        pfEnabled={pfEnabled} pfLoading={pfLoading}
        tunEnabled={tunEnabled} tunLoading={tunLoading}
        onEnablePf={() => enablePf(setError)}
        onDisablePf={() => disablePf(setError)}
        onEnableTun={() => enableTun(setError)}
      />
    </div>
  );
}

export default App;
