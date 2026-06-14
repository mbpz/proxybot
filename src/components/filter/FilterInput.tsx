import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FilterPreset, ParseResult } from "./types";
import { SavePresetButton } from "./SavePresetButton";

interface FilterInputProps {
  value: string;
  onChange: (value: string) => void;
  presets?: FilterPreset[];
  onSelectPreset?: (preset: FilterPreset) => void;
  /** Called after a successful save so the parent can refresh presets. */
  onPresetsChange?: () => void;
}

/**
 * Top-bar filter input that combines:
 *  - free-text DSL expression (with debounced parse validation)
 *  - preset dropdown selector
 *  - "Save Preset" button (opens modal)
 *
 * Validation is debounced 250ms so we don't fire a Tauri call on
 * every keystroke.
 */
export function FilterInput({
  value,
  onChange,
  presets = [],
  onSelectPreset,
  onPresetsChange,
}: FilterInputProps) {
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!value.trim()) {
      setError(null);
      return;
    }
    const handle = setTimeout(async () => {
      try {
        const result = await invoke<ParseResult>("parse_filter", { expr: value });
        if (!result.ok) {
          setError(result.error ?? "Invalid expression");
        } else {
          setError(null);
        }
      } catch (e) {
        setError(String(e));
      }
    }, 250);
    return () => clearTimeout(handle);
  }, [value]);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange(e.target.value);
    },
    [onChange]
  );

  return (
    <div className="flex items-center gap-2 px-4 py-2 bg-surface-secondary border-b border-border">
      <input
        type="text"
        value={value}
        onChange={handleChange}
        placeholder="method:GET AND host:*.example.com"
        data-testid="filter-input"
        className={`flex-1 px-2 py-1 text-sm border rounded font-mono ${
          error ? "border-red-500" : "border-border"
        }`}
      />
      {error && (
        <span
          className="text-red-500 text-xs whitespace-nowrap"
          data-testid="filter-error"
        >
          {error}
        </span>
      )}
      {presets.length > 0 && (
        <select
          value=""
          onChange={(e) => {
            const id = e.target.value;
            const p = presets.find((x) => x.id === id);
            if (p && onSelectPreset) onSelectPreset(p);
            e.currentTarget.value = "";
          }}
          data-testid="filter-preset-select"
          className="text-xs border rounded px-2 py-1"
        >
          <option value="">Load Preset...</option>
          {presets.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      )}
      <SavePresetButton currentExpr={value} onSaved={onPresetsChange} />
    </div>
  );
}