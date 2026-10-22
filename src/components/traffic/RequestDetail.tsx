import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Copy, Download, Plus, RotateCcw } from "lucide-react";
import { HeadersView } from "./HeadersView";
import { BodyView } from "./BodyView";
import { WsFramesView } from "../ws-frames/WsFramesView";
import { CodeExport } from "../shared/CodeExport";
import { MethodBadge, Badge } from "../ui/Badge";
import { Tabs } from "../ui/Tabs";
import { Button } from "../ui/Button";
import { InlineNotice } from "../ui/states";
import { generateCurl } from "../shared/codeGenerators";
import { desktop } from "../../desktop/contract";
import { getStatusTailwindClass } from "../../utils";
import type { TrafficListItem } from "./model";

interface RequestDetailProps {
  request: TrafficListItem;
}

type TabType = "headers" | "body" | "ws";
type PendingAction = "rule" | "har" | "curl" | null;

export function RequestDetail({ request }: RequestDetailProps) {
  const navigate = useNavigate();
  const [activeTab, setActiveTab] = useState<TabType>("headers");
  const [pending, setPending] = useState<PendingAction>(null);
  const [notice, setNotice] = useState<{
    variant: "success" | "error" | "info";
    message: string;
  } | null>(null);
  const isWebSocket = request.isWebSocket;

  const tabs = [
    { id: "headers", label: "Headers" },
    { id: "body", label: "Body" },
    ...(isWebSocket ? [{ id: "ws", label: "WebSocket Frames" }] : []),
  ];

  const statusClass = getStatusTailwindClass(request.status ?? undefined);
  const fullUrl = request.host.includes("://")
    ? `${request.host}${request.path}`
    : `https://${request.host}${request.path}`;

  async function copyCurl() {
    setPending("curl");
    setNotice(null);
    try {
      await navigator.clipboard.writeText(
        generateCurl(request.method, fullUrl, request.headers, request.body ?? ""),
      );
      setNotice({ variant: "success", message: "cURL command copied" });
    } catch (error) {
      setNotice({ variant: "error", message: `Copy failed: ${String(error)}` });
    } finally {
      setPending(null);
    }
  }

  /**
   * Per-request replay entry point. Opens the Replay workspace with this
   * host preselected; the replay command itself stays behind the generated
   * Desktop Contract boundary in the Replay page.
   */
  function replayRequest() {
    navigate(`/replay?host=${encodeURIComponent(request.host)}`);
  }

  async function addBreakpointRule() {
    setPending("rule");
    setNotice(null);
    try {
      await desktop.call("save_rule", {
        rule: {
          pattern: "DOMAIN-SUFFIX",
          value: request.host,
          action: { type: "BREAKPOINT", target: "REQUEST" },
          name: `Breakpoint ${request.host}`,
          priority: 100,
          enabled: true,
          comment: "Breakpoint rule created from a captured request",
        },
        filename: "custom.yaml",
        originalRule: null,
      });
      setNotice({
        variant: "success",
        message: `Breakpoint rule added: DOMAIN-SUFFIX ${request.host}. Manage it in Rules.`,
      });
    } catch (error) {
      setNotice({ variant: "error", message: `Could not add rule: ${String(error)}` });
    } finally {
      setPending(null);
    }
  }

  async function exportHar() {
    setPending("har");
    setNotice(null);
    try {
      const sessionName = `request-${request.id}-${Date.now()}`;
      const har = await desktop.call("export_har", { sessionName });
      const path = await desktop.call("save_har_file", {
        harJson: JSON.stringify(har),
        sessionName,
      });
      setNotice({ variant: "success", message: `Session HAR exported to: ${path}` });
    } catch (error) {
      setNotice({ variant: "error", message: `HAR export failed: ${String(error)}` });
    } finally {
      setPending(null);
    }
  }

  return (
    <div className="h-full flex flex-col bg-surface-secondary">
      {/* Header */}
      <div className="px-4 py-3 bg-surface-tertiary border-b border-border">
        <div className="flex items-center gap-2 mb-2">
          <MethodBadge method={request.method} />
          <span className="font-mono text-sm text-text-primary truncate">
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
            Duration: {request.durationMs ?? "-"}ms
          </span>
          {request.appName && <Badge variant="info">{request.appName}</Badge>}
        </div>

        {/* Inline request actions: the core "inspect -> act" loop. */}
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <Button
            variant="primary"
            size="sm"
            disabled={pending !== null}
            onClick={replayRequest}
          >
            <RotateCcw size={14} /> Replay this request
          </Button>
          <Button
            variant="secondary"
            size="sm"
            disabled={pending !== null}
            onClick={() => void addBreakpointRule()}
          >
            <Plus size={14} /> {pending === "rule" ? "Adding..." : "Add breakpoint rule"}
          </Button>
          <Button variant="secondary" size="sm" disabled={pending !== null} onClick={() => void copyCurl()}>
            <Copy size={14} /> Copy as cURL
          </Button>
          <Button variant="secondary" size="sm" disabled={pending !== null} onClick={() => void exportHar()}>
            <Download size={14} /> {pending === "har" ? "Exporting..." : "Export session HAR"}
          </Button>
        </div>

        {notice && (
          <div className="mt-3">
            <InlineNotice variant={notice.variant} onDismiss={() => setNotice(null)}>
              {notice.message}
            </InlineNotice>
          </div>
        )}

        <div className="mt-3">
          <CodeExport
            method={request.method}
            url={fullUrl}
            headers={request.headers}
            body={request.body ?? undefined}
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
        {activeTab === "body" && <BodyView body={request.body ?? undefined} />}
        {activeTab === "ws" && isWebSocket && <WsFramesView requestId={request.id} />}
      </div>
    </div>
  );
}
