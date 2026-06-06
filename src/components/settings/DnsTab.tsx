import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { safeInvoke } from "../../utils/safeInvoke";
import { Button } from "../ui/Button";
import { Globe, RefreshCw } from "lucide-react";

interface DnsUpstream {
  upstream_type: string;
  address: string;
}

export function DnsTab() {
  const [upstream, setUpstream] = useState<DnsUpstream | null>(null);
  const [upstreamType, setUpstreamType] = useState("plainudp");
  const [address, setAddress] = useState("");
  const [saving, setSaving] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    load();
  }, []);

  async function load() {
    const result = await safeInvoke<DnsUpstream>("get_dns_upstream");
    if (result) {
      setUpstream(result);
      setUpstreamType(result.upstream_type);
      setAddress(result.address);
    }
  }

  async function save() {
    try {
      setSaving(true);
      setMessage("");
      await invoke("set_dns_upstream", {
        upstream: { upstream_type: upstreamType, address },
      });
      setUpstream({ upstream_type: upstreamType, address });
      setMessage("DNS upstream saved");
    } catch (e) {
      setMessage(`Error: ${e}`);
    } finally {
      setSaving(false);
    }
  }

  async function reloadLists() {
    try {
      setReloading(true);
      await invoke("reload_dns_lists");
      setMessage("DNS lists reloaded");
    } catch (e) {
      setMessage(`Error: ${e}`);
    } finally {
      setReloading(false);
    }
  }

  const hasChanges =
    upstream &&
    (upstreamType !== upstream.upstream_type || address !== upstream.address);

  return (
    <div className="space-y-4">
      {/* Upstream DNS */}
      <div className="card">
        <div className="flex items-center gap-3 mb-4">
          <Globe size={20} className="text-text-secondary" />
          <div className="font-medium text-text-primary">Upstream DNS</div>
        </div>

        <div className="space-y-3">
          <div>
            <label className="text-sm text-text-muted block mb-1">Type</label>
            <select
              value={upstreamType}
              onChange={(e) => setUpstreamType(e.target.value)}
              className="w-full"
            >
              <option value="plainudp">Plain UDP (e.g., 8.8.8.8:53)</option>
              <option value="doh">DNS over HTTPS (DoH)</option>
            </select>
          </div>

          <div>
            <label className="text-sm text-text-muted block mb-1">
              {upstreamType === "doh" ? "DoH URL" : "DNS Server"}
            </label>
            <input
              type="text"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder={
                upstreamType === "doh"
                  ? "https://1.1.1.1/dns-query"
                  : "8.8.8.8:53"
              }
              className="w-full"
            />
          </div>

          <div className="flex items-center gap-2">
            <Button
              variant="primary"
              size="sm"
              disabled={saving || !hasChanges}
              onClick={save}
            >
              {saving ? "Saving..." : "Save"}
            </Button>
            {message && (
              <span className="text-xs text-text-muted">{message}</span>
            )}
          </div>
        </div>
      </div>

      {/* Reload Lists */}
      <div className="card">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <RefreshCw size={20} className="text-text-secondary" />
            <div>
              <div className="font-medium text-text-primary">Reload DNS Lists</div>
              <div className="text-sm text-text-muted">
                Reload hosts file and domain blocklist
              </div>
            </div>
          </div>
          <Button
            variant="secondary"
            size="sm"
            disabled={reloading}
            onClick={reloadLists}
          >
            {reloading ? "Reloading..." : "Reload"}
          </Button>
        </div>
      </div>
    </div>
  );
}
