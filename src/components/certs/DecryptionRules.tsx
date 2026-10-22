import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ShieldOff, Trash2, Plus, Lock, EyeOff } from "lucide-react";
import { Button } from "../ui/Button";

/** Mirror of `db::TlsRuleRow` on the Rust side. */
interface TlsRuleRow {
  id: number;
  pattern: string;
  action: TlsAction;
  hit_count: number;
  sort_order: number;
}

type TlsAction = "Decrypt" | "Bypass" | "Passthrough";

const ACTION_META: Record<
  TlsAction,
  { label: string; hint: string; icon: typeof Lock; cls: string }
> = {
  Decrypt: {
    label: "Decrypt",
    hint: "MITM and capture (default)",
    icon: Lock,
    cls: "border-accent-blue/40 bg-[rgba(77,157,224,0.15)] text-accent-blue",
  },
  Bypass: {
    label: "Bypass",
    hint: "Tunnel raw, log metadata only — for cert-pinned apps",
    icon: ShieldOff,
    cls: "border-accent-yellow/40 bg-[rgba(244,211,94,0.12)] text-accent-yellow",
  },
  Passthrough: {
    label: "Passthrough",
    hint: "Tunnel raw, capture nothing — for noisy telemetry",
    icon: EyeOff,
    cls: "border-border bg-surface-tertiary text-text-secondary",
  },
};

/**
 * Per-host TLS decryption rules. First-match-wins, ordered by the
 * backend's sort_order (new rules append to the end). Lets users
 * carve cert-pinned apps and telemetry out of the MITM path without
 * editing rules.yaml or restarting the proxy.
 */
export function DecryptionRules() {
  const [rules, setRules] = useState<TlsRuleRow[]>([]);
  const [pattern, setPattern] = useState("");
  const [action, setAction] = useState<TlsAction>("Bypass");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    load();
  }, []);

  async function load() {
    try {
      setRules((await invoke<TlsRuleRow[] | null>("get_tls_rules")) ?? []);
    } catch (err) {
      setError(String(err));
    }
  }

  async function add() {
    const p = pattern.trim();
    if (!p) return;
    setBusy(true);
    setError(null);
    try {
      // The command returns the refreshed rule list, so we don't
      // need a second round-trip to re-read.
      const next = await invoke<TlsRuleRow[] | null>("add_tls_rule", {
        pattern: p,
        action,
      });
      setRules(next ?? []);
      setPattern("");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function remove(id: number) {
    setError(null);
    try {
      const next = await invoke<TlsRuleRow[] | null>("delete_tls_rule", { id });
      setRules(next ?? []);
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="card max-w-2xl mt-6">
      <h2 className="text-lg font-semibold mb-1">Decryption rules</h2>
      <p className="text-sm text-text-muted mb-4">
        Choose which hosts get TLS decryption. Bypass cert-pinned apps
        (WeChat, Alipay, banking) so they don't crash; Passthrough
        noisy telemetry so it never enters the capture. First match
        wins — add specific Decrypt rules before broad wildcards.
      </p>

      {error && (
        <div className="error-banner mb-4">
          <span className="error-banner-message">{error}</span>
        </div>
      )}

      <div className="flex gap-2 mb-4">
        <input
          className="input flex-1 font-mono text-sm"
          placeholder="*.weixin.qq.com"
          value={pattern}
          onChange={(e) => setPattern(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") add();
          }}
        />
        <select
          className="input"
          value={action}
          onChange={(e) => setAction(e.target.value as TlsAction)}
        >
          <option value="Decrypt">Decrypt</option>
          <option value="Bypass">Bypass</option>
          <option value="Passthrough">Passthrough</option>
        </select>
        <Button variant="primary" size="sm" onClick={add} disabled={busy || !pattern.trim()}>
          <Plus size={16} />
          Add
        </Button>
      </div>

      <p className="text-xs text-text-muted mb-3">{ACTION_META[action].hint}</p>

      {rules.length === 0 ? (
        <p className="text-sm text-text-muted">
          No rules — every host is decrypted.
        </p>
      ) : (
        <ul className="space-y-1">
          {rules.map((r) => {
            const meta = ACTION_META[r.action];
            const Icon = meta.icon;
            return (
              <li
                key={r.id}
                className="flex items-center gap-3 px-3 py-2 rounded border border-border"
              >
                <span className="font-mono text-sm flex-1 truncate">{r.pattern}</span>
                <span
                  className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs border ${meta.cls}`}
                >
                  <Icon size={12} />
                  {meta.label}
                </span>
                <span className="text-xs text-text-muted w-16 text-right">
                  {r.hit_count} hits
                </span>
                <button
                  className="text-text-muted hover:text-red-500"
                  onClick={() => remove(r.id)}
                  aria-label={`Delete rule ${r.pattern}`}
                >
                  <Trash2 size={14} />
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
