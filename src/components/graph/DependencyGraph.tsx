import { useEffect, useRef, useMemo } from "react";
import { Network, DataSet } from "vis-network";

interface RequestNode {
  id: string;
  host: string;
  path: string;
  method: string;
  status?: number;
  duration_ms: number;
  timestamp: number;
}

interface Edge {
  from: string;
  to: string;
}

interface GraphData {
  requests: RequestNode[];
  edges: Edge[];
}

interface DependencyGraphProps {
  data: GraphData | null;
}

function getStatusColor(status?: number): string {
  if (!status) return "#6b7280";
  if (status >= 200 && status < 300) return "#10b981";
  if (status >= 400 && status < 500) return "#f59e0b";
  if (status >= 500) return "#ef4444";
  return "#3b82f6";
}

export function DependencyGraph({ data }: DependencyGraphProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const networkRef = useRef<Network | null>(null);

  const { nodes, edges } = useMemo(() => {
    if (!data?.requests) {
      return { nodes: new DataSet([]), edges: new DataSet([]) };
    }

    const nodesData = data.requests.slice(0, 30).map((req) => ({
      id: req.id,
      label: `${req.method} ${req.path.slice(0, 15)}`,
      color: getStatusColor(req.status),
      title: `${req.host}${req.path}\n${req.duration_ms}ms`,
    }));

    const edgesData = data.edges.slice(0, 50).map((e, i) => ({
      id: i,
      from: e.from,
      to: e.to,
    }));

    return {
      nodes: new DataSet(nodesData),
      edges: new DataSet(edgesData),
    };
  }, [data]);

  useEffect(() => {
    if (!containerRef.current) return;

    const options = {
      physics: {
        enabled: true,
        solver: "forceAtlas2Based",
        forceAtlas2Based: {
          theta: 0.5,
          gravitationalConstant: -50,
        },
      },
      edges: {
        arrows: "to",
        color: { color: "#94a3b8", highlight: "#3b82f6" },
      },
      nodes: {
        shape: "box",
        font: { size: 12 },
      },
    };

    networkRef.current = new Network(containerRef.current, { nodes, edges }, options);

    return () => {
      networkRef.current?.destroy();
    };
  }, [nodes, edges]);

  if (!data?.requests?.length) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No dependency data available
      </div>
    );
  }

  return <div ref={containerRef} className="w-full h-full" />;
}