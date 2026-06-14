import { useState } from "react";
import { HeadersView } from "./HeadersView";
import { BodyView } from "./BodyView";
import { WsFramesView } from "../ws-frames/WsFramesView";
import { CodeExport } from "../shared/CodeExport";
import { MethodBadge, Badge } from "../ui/Badge";
import { Tabs } from "../ui/Tabs";
import { getStatusTailwindClass } from "../../utils";

interface InterceptedRequest {
  id: string;
  method: string;
  host: string;
  path: string;
  status?: number;
  duration_ms: number;
  headers: Record<string, string>;
  body?: string;
  app_tag?: string;
  is_websocket?: boolean;
}

interface RequestDetailProps {
  request: InterceptedRequest;
}

type TabType = "headers" | "body" | "ws";

export function RequestDetail({ request }: RequestDetailProps) {
  const [activeTab, setActiveTab] = useState<TabType>("headers");
  const isWebSocket = request.is_websocket ?? false;

  const tabs = [
    { id: "headers", label: "Headers" },
    { id: "body", label: "Body" },
    ...(isWebSocket ? [{ id: "ws", label: "WebSocket Frames" }] : []),
  ];

  const statusClass = getStatusTailwindClass(request.status);

  return (
    <div className="h-full flex flex-col bg-surface-secondary">
      {/* Header */}
      <div className="px-4 py-3 bg-surface-tertiary border-b border-border">
        <div className="flex items-center gap-2 mb-2">
          <MethodBadge method={request.method} />
          <span className="font-mono text-sm text-text-primary">
            {request.host}
            <span className="text-text-secondary">{request.path}</span>
          </span>
        </div>

        <div className="flex items-center gap-4 text-xs">
          <span>
            Status:{" "}
            <span className={`font-mono ${statusClass}`}>
              {request.status || ".."}
            </span>
          </span>
          <span className="text-text-muted">
            Duration: {request.duration_ms}ms
          </span>
          {request.app_tag && <Badge variant="info">{request.app_tag}</Badge>}
        </div>

        <div className="mt-3">
          <CodeExport
            method={request.method}
            url={`${request.host}${request.path}`}
            headers={request.headers}
            body={request.body}
          />
        </div>
      </div>

      {/* Tabs */}
      <Tabs
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={(id) => setActiveTab(id as TabType)}
      />

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {activeTab === "headers" && <HeadersView headers={request.headers} />}
        {activeTab === "body" && <BodyView body={request.body} />}
        {activeTab === "ws" && isWebSocket && <WsFramesView requestId={request.id} />}
      </div>
    </div>
  );
}
