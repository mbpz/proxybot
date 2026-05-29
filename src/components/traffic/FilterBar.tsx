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
    <div className="flex items-center gap-3 px-4 py-2 bg-surface-secondary border-b border-border">
      {/* Method filter */}
      <select
        value={filters.method || ""}
        onChange={(e) =>
          onChange({ ...filters, method: e.target.value || undefined })
        }
        className="font-mono text-xs"
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
        className="font-mono text-xs"
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
        className="font-mono text-xs w-48"
      />

      {/* Search */}
      <input
        type="text"
        placeholder="Search path..."
        value={filters.search || ""}
        onChange={(e) =>
          onChange({ ...filters, search: e.target.value || undefined })
        }
        className="font-mono text-xs flex-1"
      />

      {/* Clear button */}
      {hasFilters && (
        <button
          onClick={() => onChange({})}
          className="btn btn-ghost btn-sm text-xs"
        >
          Clear
        </button>
      )}
    </div>
  );
}
