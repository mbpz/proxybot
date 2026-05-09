import { useState, useCallback } from "react";

interface FilterPreset {
  id: string;
  name: string;
  expr: string;
}

interface FilterInputProps {
  value: string;
  onChange: (value: string) => void;
  presets?: FilterPreset[];
  onSavePreset?: (name: string, expr: string) => void;
  error?: string | null;
}

export function FilterInput({
  value,
  onChange,
  presets = [],
  onSavePreset,
  error,
}: FilterInputProps) {
  const [showPresets, setShowPresets] = useState(false);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange(e.target.value);
    },
    [onChange]
  );

  const handlePresetSelect = useCallback(
    (expr: string) => {
      onChange(expr);
      setShowPresets(false);
    },
    [onChange]
  );

  return (
    <div className="relative">
      <input
        type="text"
        value={value}
        onChange={handleChange}
        placeholder="method:GET AND status:2*"
        className={`w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500 ${
          error ? "border-red-500" : ""
        }`}
      />
      {error && <span className="text-red-500 text-sm mt-1">{error}</span>}

      <div className="flex gap-2 mt-2">
        {presets.length > 0 && (
          <select
            onChange={(e) => handlePresetSelect(e.target.value)}
            className="px-2 py-1 border rounded text-sm"
          >
            <option value="">Load Preset...</option>
            {presets.map((p) => (
              <option key={p.id} value={p.expr}>
                {p.name}
              </option>
            ))}
          </select>
        )}

        {onSavePreset && value && (
          <button
            onClick={() => {
              const name = prompt("Preset name:");
              if (name) onSavePreset(name, value);
            }}
            className="px-2 py-1 bg-gray-100 rounded text-sm hover:bg-gray-200"
          >
            Save Preset
          </button>
        )}
      </div>
    </div>
  );
}