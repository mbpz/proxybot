import { useRef, useState } from "react";
import { Save } from "lucide-react";
import { desktop } from "../../desktop/contract";
import { Button } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { InlineNotice } from "../ui/states";

interface SavePresetButtonProps {
  currentExpr: string;
  onSaved?: () => void;
}

/**
 * Button that opens an accessible modal asking for a preset name, then calls
 * the `save_filter_preset` Tauri command. Hidden when the input is empty.
 */
export function SavePresetButton({ currentExpr, onSaved }: SavePresetButtonProps) {
  const [showDialog, setShowDialog] = useState(false);
  const [name, setName] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

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
    } catch (cause) {
      setError(String(cause));
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

      <Dialog
        open={showDialog}
        title="Save Filter Preset"
        onClose={() => setShowDialog(false)}
        initialFocusRef={inputRef}
        footer={
          <>
            <button
              type="button"
              className="btn btn-ghost btn-sm"
              onClick={() => setShowDialog(false)}
            >
              Cancel
            </button>
            <button
              type="button"
              className="btn btn-primary btn-sm"
              onClick={() => void handleSave()}
              disabled={saving || !name.trim()}
              data-testid="filter-save-preset-confirm"
            >
              {saving ? "Saving..." : "Save"}
            </button>
          </>
        }
      >
        <div data-testid="filter-save-preset-dialog" className="space-y-3">
          {error && <InlineNotice variant="error">{error}</InlineNotice>}
          <label htmlFor="preset-name" className="block text-sm text-text-muted">
            Preset name
          </label>
          <input
            ref={inputRef}
            id="preset-name"
            type="text"
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Preset name"
            className="w-full"
            data-testid="filter-save-preset-name"
            onKeyDown={(event) => {
              if (event.key === "Enter" && name.trim() && !saving) void handleSave();
            }}
          />
          <p className="break-all font-mono text-xs text-text-muted">{currentExpr}</p>
        </div>
      </Dialog>
    </>
  );
}
