interface FilterState {
  method?: string;
  host?: string;
  status?: number;
  appTag?: string;
  search?: string;
}

interface FilterBarProps {
  filters: FilterState;
  onChange: (filters: FilterState) => void;
}

export function FilterBar({ filters, onChange }: FilterBarProps) {
  const hasFilters = filters.method || filters.host || filters.search || filters.appTag;

  return (
    <div
      className="flex items-center gap-3 px-4 py-2"
      style={{
        background: "var(--bg-secondary)",
        borderBottom: "1px solid var(--border)",
      }}
    >
      {/* Method filter */}
      <select
        value={filters.method || ""}
        onChange={(e) =>
          onChange({ ...filters, method: e.target.value || undefined })
        }
        style={{
          background: "var(--bg-tertiary)",
          border: "1px solid var(--border)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-1) var(--space-2)",
          fontSize: "var(--text-xs)",
          color: "var(--text-primary)",
          fontFamily: "var(--font-mono)",
        }}
      >
        <option value="">All Methods</option>
        <option value="GET">GET</option>
        <option value="POST">POST</option>
        <option value="PUT">PUT</option>
        <option value="DELETE">DELETE</option>
        <option value="PATCH">PATCH</option>
      </select>

      {/* App filter */}
      <select
        value={filters.appTag || ""}
        onChange={(e) =>
          onChange({ ...filters, appTag: e.target.value || undefined })
        }
        style={{
          background: "var(--bg-tertiary)",
          border: "1px solid var(--border)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-1) var(--space-2)",
          fontSize: "var(--text-xs)",
          color: "var(--text-primary)",
          fontFamily: "var(--font-mono)",
        }}
      >
        <option value="">All Apps</option>
        <option value="WeChat">WeChat</option>
        <option value="Douyin">Douyin</option>
        <option value="Alipay">Alipay</option>
        <option value="unknown">Unknown</option>
      </select>

      {/* Host filter */}
      <input
        type="text"
        placeholder="host:*.example.com"
        value={filters.host || ""}
        onChange={(e) =>
          onChange({ ...filters, host: e.target.value || undefined })
        }
        style={{
          background: "var(--bg-tertiary)",
          border: "1px solid var(--border)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-1) var(--space-2)",
          fontSize: "var(--text-xs)",
          color: "var(--text-primary)",
          fontFamily: "var(--font-mono)",
          width: "200px",
        }}
      />

      {/* Search */}
      <input
        type="text"
        placeholder="Search path..."
        value={filters.search || ""}
        onChange={(e) =>
          onChange({ ...filters, search: e.target.value || undefined })
        }
        style={{
          background: "var(--bg-tertiary)",
          border: "1px solid var(--border)",
          borderRadius: "var(--radius-md)",
          padding: "var(--space-1) var(--space-2)",
          fontSize: "var(--text-xs)",
          color: "var(--text-primary)",
          fontFamily: "var(--font-mono)",
          flex: 1,
        }}
      />

      {/* Clear button */}
      {hasFilters && (
        <button
          onClick={() => onChange({})}
          className="btn btn-ghost btn-sm"
          style={{ fontSize: "var(--text-xs)" }}
        >
          Clear
        </button>
      )}
    </div>
  );
}