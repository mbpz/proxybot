import { useMemo } from "react";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
} from "recharts";

interface RequestNode {
  id: string;
  host: string;
  path: string;
  method: string;
  status?: number;
  duration_ms: number;
  timestamp: number;
}

interface GraphData {
  requests: RequestNode[];
  edges: any[];
}

interface WaterfallChartProps {
  data: GraphData | null;
}

function getStatusColor(status?: number): string {
  if (!status) return "#6b7280";
  if (status >= 200 && status < 300) return "#10b981";
  if (status >= 300 && status < 400) return "#3b82f6";
  if (status >= 400 && status < 500) return "#f59e0b";
  if (status >= 500) return "#ef4444";
  return "#6b7280";
}

export function WaterfallChart({ data }: WaterfallChartProps) {
  const chartData = useMemo(() => {
    if (!data?.requests) return [];
    return data.requests.slice(0, 50).map((req) => ({
      id: req.id,
      name: `${req.method} ${req.path.slice(0, 20)}`,
      duration: req.duration_ms,
      timestamp: req.timestamp,
      status: req.status,
      color: getStatusColor(req.status),
    }));
  }, [data]);

  if (!data?.requests?.length) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No request data available
      </div>
    );
  }

  return (
    <div className="w-full h-full p-4">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={chartData} layout="vertical">
          <XAxis type="number" label="Duration (ms)" />
          <YAxis
            type="category"
            dataKey="name"
            width={150}
            fontSize={10}
          />
          <Tooltip
            formatter={(_value, _name, props) => [
              `${props.payload.duration}ms`,
              "Duration",
            ]}
            labelFormatter={(label, payload) => {
              if (payload?.[0]) {
                return `${payload[0].payload.name}`;
              }
              return label;
            }}
          />
          <Bar dataKey="duration" fill="#3b82f6" />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}