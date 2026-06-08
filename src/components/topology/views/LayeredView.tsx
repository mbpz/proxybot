import { useEffect, useRef } from "react";
import { Network, Node, Edge, DataSet } from "vis-network";
import { TopologyGraph } from "../types";
import { nodeBackgroundColor, nodeBorderColor, nodeSize } from "../nodeColor";

interface Props {
  graph: TopologyGraph;
  onNodeClick: (nodeId: string) => void;
}

export function LayeredView({ graph, onNodeClick }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const networkRef = useRef<Network | null>(null);

  useEffect(() => {
    if (!ref.current) return;

    // Build layered view: synthesize Proxy + App nodes from data
    const proxyNode: Node = {
      id: "proxy:main",
      label: "PROXY",
      color: { background: "rgba(168,85,247,0.15)", border: "#a855f7" },
      size: 30,
    };
    const appNodes: Node[] = Array.from(
      new Set(graph.nodes.map((n) => n.app_tag).filter((t): t is string => !!t)),
    ).map((tag) => ({
      id: `app:${tag}`,
      label: tag,
      color: { background: "rgba(34,197,94,0.12)", border: "#22c55e" },
      size: 20,
    }));

    const dataNodes: Node[] = graph.nodes
      .filter((n) => n.kind === "device" || n.kind === "host")
      .map((n) => ({
        id: n.id,
        label: n.label,
        color: { background: nodeBackgroundColor(n), border: nodeBorderColor(n) },
        size: nodeSize(n),
      }));

    // Split device->host edges into device->app and app->host
    const layeredEdges: Edge[] = graph.edges.flatMap((e) => {
      const host = graph.nodes.find((n) => n.id === e.to);
      const appTag = host?.app_tag;
      if (!appTag) return [];
      return [
        {
          id: `${e.id}->app`,
          from: e.from,
          to: `app:${appTag}`,
          color: { color: e.is_anomalous ? "#ff4d4d" : "#8888aa" },
        },
        {
          id: `${e.id}->host`,
          from: `app:${appTag}`,
          to: e.to,
          color: { color: e.is_anomalous ? "#ff4d4d" : "#8888aa" },
        },
      ];
    });

    // Add device->proxy and proxy->app edges
    const deviceIds = graph.nodes.filter((n) => n.kind === "device").map((n) => n.id);
    const proxyEdges: Edge[] = deviceIds.map((id) => ({
      id: `${id}->proxy`,
      from: id,
      to: "proxy:main",
      color: { color: "#a855f7" },
    }));

    const nodes = new DataSet<Node>([proxyNode, ...appNodes, ...dataNodes]);
    const edges = new DataSet<Edge>([...layeredEdges, ...proxyEdges]);

    networkRef.current = new Network(
      ref.current,
      { nodes, edges },
      {
        layout: {
          hierarchical: { enabled: true, direction: "LR", sortMethod: "directed" },
        },
        physics: { enabled: false },
        interaction: { hover: true, zoomView: true, dragView: true },
      },
    );
    networkRef.current.on("click", (params) => {
      if (params.nodes.length > 0) onNodeClick(params.nodes[0] as string);
    });

    return () => {
      networkRef.current?.destroy();
      networkRef.current = null;
    };
  }, [graph, onNodeClick]);

  return <div ref={ref} className="w-full h-full" />;
}
