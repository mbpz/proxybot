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
  return (
    <div className="flex gap-2 p-2 bg-gray-100 border-b">
      <select
        value={filters.method || ""}
        onChange={(e) => onChange({ ...filters, method: e.target.value || undefined })}
        className="px-2 py-1 border rounded"
      >
        <option value="">All Methods</option>
        <option value="GET">GET</option>
        <option value="POST">POST</option>
        <option value="PUT">PUT</option>
        <option value="DELETE">DELETE</option>
        <option value="PATCH">PATCH</option>
      </select>

      <input
        type="text"
        placeholder="host:*.example.com"
        value={filters.host || ""}
        onChange={(e) => onChange({ ...filters, host: e.target.value || undefined })}
        className="px-2 py-1 border rounded flex-1"
      />

      <input
        type="text"
        placeholder="Search..."
        value={filters.search || ""}
        onChange={(e) => onChange({ ...filters, search: e.target.value || undefined })}
        className="px-2 py-1 border rounded flex-1"
      />

      <button
        onClick={() => onChange({})}
        className="px-3 py-1 bg-gray-200 rounded hover:bg-gray-300"
      >
        Clear
      </button>
    </div>
  );
}