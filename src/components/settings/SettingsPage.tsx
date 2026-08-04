import { useState } from "react";
import { Tabs } from "../ui/Tabs";
import { GeneralTab } from "./GeneralTab";
import { NetworkTab } from "./NetworkTab";
import { DnsTab } from "./DnsTab";
import { AboutTab } from "./AboutTab";
import { Settings } from "lucide-react";

type SettingsTab = "general" | "network" | "dns" | "about";

const tabs = [
  { id: "general", label: "General" },
  { id: "network", label: "Network" },
  { id: "dns", label: "DNS" },
  { id: "about", label: "About" },
];

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState<SettingsTab>("general");

  return (
    <div className="panel">
      <div className="panel-header">
        <div className="flex items-center gap-2">
          <Settings size={18} className="text-text-secondary" />
          <span className="panel-title">Settings</span>
        </div>
      </div>

      <Tabs
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={(id) => setActiveTab(id as SettingsTab)}
      />

      <div className="panel-body">
        {activeTab === "general" && <GeneralTab />}
        {activeTab === "network" && <NetworkTab />}
        {activeTab === "dns" && <DnsTab />}
        {activeTab === "about" && <AboutTab />}
      </div>
    </div>
  );
}
