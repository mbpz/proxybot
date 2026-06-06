// ============================================================
// AI Panel
// ============================================================

import { useState } from "react";
import { Tabs } from "../ui/Tabs";
import { ErrorBoundary } from "../ui/error-boundary";
import { Brain } from "lucide-react";
import { TokenUsageTab } from "./TokenUsageTab";
import { ApiInferenceTab } from "./ApiInferenceTab";
import { AuthFlowTab } from "./AuthFlowTab";
import { VisionTab } from "./VisionTab";

export function AiPage() {
  const [activeTab, setActiveTab] = useState("token");

  const tabs = [
    { id: "token", label: "Token Usage" },
    { id: "inference", label: "API Inference" },
    { id: "auth", label: "Auth Flow" },
    { id: "vision", label: "Vision" },
  ];

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6 flex items-center gap-2">
        <Brain size={24} className="text-accent-purple" />
        AI Analysis
      </h1>

      <div className="panel">
        <Tabs tabs={tabs} activeTab={activeTab} onTabChange={setActiveTab} />
        <div className="panel-body">
          <ErrorBoundary>
            {activeTab === "token" && <TokenUsageTab />}
            {activeTab === "inference" && <ApiInferenceTab />}
            {activeTab === "auth" && <AuthFlowTab />}
            {activeTab === "vision" && <VisionTab />}
          </ErrorBoundary>
        </div>
      </div>
    </div>
  );
}
