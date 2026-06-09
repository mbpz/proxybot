import { useEffect, useRef } from "react";
import { Network, Node, Edge } from "vis-network";
import { DataSet } from "vis-data";
import { TopologyGraph } from "../types";
import { nodeBackgroundColor, nodeBorderColor, nodeSize } from "../nodeColor";

interface Props {
  graph: TopologyGraph;
  onNodeClick: (nodeId: string) => void;
}

export function GroupedView({ graph, onNodeClick }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const networkRef = useRef<Network | null>(null);
  const onNodeClickRef = useRef(onNodeClick);
  onNodeClickRef.current = onNodeClick;

  useEffect(() => {
    if (!ref.current) return;

    const nodes = new DataSet<Node>(
      graph.nodes.map((n) => ({
        id: n.id,
        label: n.label,
        group: n.app_tag || n.kind,
        color: { background: nodeBackgroundColor(n), border: nodeBorderColor(n) },
        size: nodeSize(n),
      })),
    );

    const edges = new DataSet<Edge>(
      graph.edges.map((e) => ({
        id: e.id,
        from: e.from,
        to: e.to,
        color: { color: e.is_anomalous ? "#ff4d4d" : "#8888aa" },
        width: Math.max(1, Math.log2(e.total_bytes || 1) / 4),
      })),
    );

    networkRef.current = new Network(
      ref.current,
      { nodes, edges },
      {
        physics: {
          enabled: true,
          solver: "forceAtlas2Based",
          forceAtlas2Based: { theta: 0.5, gravitationalConstant: -50 },
        },
        interaction: { hover: true, zoomView: true, dragView: true },
      },
    );
    networkRef.current.on("click", (params) => {
      if (params.nodes.length > 0) onNodeClickRef.current(params.nodes[0] as string);
    });

    return () => {
      networkRef.current?.destroy();
      networkRef.current = null;
    };
  }, [graph]);

  return <div ref={ref} className="w-full h-full" />;
}
