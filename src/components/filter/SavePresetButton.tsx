import { useState } from "react";
import { Save } from "lucide-react";
import { desktop } from "../../desktop/contract";
import { Button } from "../ui/Button";

interface SavePresetButtonProps {
  currentExpr: string;
  onSaved?: () => void;
}

/**
 * Button that opens a modal asking for a preset name, then calls
 * the `save_filter_preset` Tauri command.
 *
 * Hidden when the input is empty (no expression to save).
 */
export function SavePresetButton({ currentExpr, onSaved }: SavePresetButtonProps) {
  const [showDialog, setShowDialog] = useState(false);
  const [name, setName] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!currentExpr.trim()) return null;

  function openDialog() {
    setName("");
    setError(null);
    setShowDialog(true);
  }

  async function handleSave() {
    if (!name.trim()) {
      setError("Name is required");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await desktop.call("save_filter_preset", {
        preset: {
          id: crypto.randomUUID(),
          name: name.trim(),
          expr: currentExpr.trim(),
        },
      });
      setShowDialog(false);
      onSaved?.();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <Button
        variant="secondary"
        size="sm"
        onClick={openDialog}
        data-testid="filter-save-preset"
      >
        <Save size={14} /> Save
      </Button>

      {showDialog && (
        <div
          className="fixed inset-0 bg-black/40 flex items-center justify-center z-50"
          data-testid="filter-save-preset-dialog"
        >
          <div className="bg-white rounded-lg p-4 w-80 space-y-3 shadow-lg">
            <h3 className="font-semibold text-sm">Save Filter Preset</h3>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Preset name"
              className="w-full border rounded px-2 py-1 text-sm"
              data-testid="filter-save-preset-name"
              onKeyDown={(e) => e.key === "Enter" && handleSave()}
              autoFocus
            />
            {error && <p className="text-red-500 text-xs">{error}</p>}
            <p className="text-xs text-gray-500 break-all font-mono">
              {currentExpr}
            </p>
            <div className="flex justify-end gap-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setShowDialog(false)}
              >
                Cancel
              </Button>
              <Button
                variant="primary"
                size="sm"
                onClick={handleSave}
                disabled={saving || !name.trim()}
                data-testid="filter-save-preset-confirm"
              >
                {saving ? "Saving..." : "Save"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
