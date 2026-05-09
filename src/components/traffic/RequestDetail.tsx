import { useState } from "react";
import { HeadersView } from "./HeadersView";
import { BodyView } from "./BodyView";
import { WsFramesView } from "./WsFramesView";

interface InterceptedRequest {
  id: string;
  method: string;
  host: string;
  path: string;
  status?: number;
  duration_ms: number;
  headers: Record<string, string>;
  body?: string;
}

interface RequestDetailProps {
  request: InterceptedRequest;
}

type TabType = "headers" | "body" | "ws";

export function RequestDetail({ request }: RequestDetailProps) {
  const [activeTab, setActiveTab] = useState<TabType>("headers");

  const tabs: { key: TabType; label: string }[] = [
    { key: "headers", label: "Headers" },
    { key: "body", label: "Body" },
    { key: "ws", label: "WS Frames" },
  ];

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b bg-gray-50">
        <div className="text-sm text-gray-500">
          {request.method} {request.host}{request.path}
        </div>
        <div className="text-sm mt-1">
          Status: <span className={request.status && request.status >= 400 ? "text-red-600" : "text-green-600"}>
            {request.status || ".."}
          </span>
          {" | "}
          Duration: {request.duration_ms}ms
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`px-4 py-2 text-sm ${
              activeTab === tab.key
                ? "border-b-2 border-blue-500 text-blue-600"
                : "text-gray-600"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {activeTab === "headers" && <HeadersView headers={request.headers} />}
        {activeTab === "body" && <BodyView body={request.body} />}
        {activeTab === "ws" && <WsFramesView requestId={request.id} />}
      </div>
    </div>
  );
}
