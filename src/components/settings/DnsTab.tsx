import { useState, useEffect } from "react";
import { desktop, type DesktopContract } from "../../desktop/contract";
import type { DnsUpstream, DnsUpstreamType } from "../../generated/desktop-contract";
import { Button } from "../ui/Button";
import { Globe, RefreshCw } from "lucide-react";

interface DnsTabProps {
  contract?: DesktopContract;
}

export function DnsTab({ contract = desktop }: DnsTabProps) {
  const [upstream, setUpstream] = useState<DnsUpstream | null>(null);
  const [upstreamType, setUpstreamType] = useState<DnsUpstreamType>("plainudp");
  const [address, setAddress] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void load();
  }, [contract]);

  async function load() {
    setLoading(true);
    setMessage("");
    setError(null);
    try {
      const result = await contract.call("get_dns_upstream", {});
      setUpstream(result);
      setUpstreamType(result.upstream_type);
      setAddress(result.address);
    } catch (cause) {
      setError(errorMessage("Could not load DNS configuration", cause));
    } finally {
      setLoading(false);
    }
  }

  async function save() {
    try {
      setSaving(true);
      setMessage("");
      setError(null);
      await contract.call("set_dns_upstream", {
        upstream: { upstream_type: upstreamType, address },
      });
      setUpstream({ upstream_type: upstreamType, address });
      setMessage("DNS upstream saved");
    } catch (e) {
      setError(errorMessage("Could not save DNS upstream", e));
    } finally {
      setSaving(false);
    }
  }

  async function reloadLists() {
    try {
      setReloading(true);
      setMessage("");
      setError(null);
      await contract.call("reload_dns_lists", {});
      setMessage("DNS lists reloaded");
    } catch (e) {
      setError(errorMessage("Could not reload DNS lists", e));
    } finally {
      setReloading(false);
    }
  }

  const hasChanges = Boolean(
    upstream &&
      (upstreamType !== upstream.upstream_type || address !== upstream.address),
  );

  return (
    <div className="space-y-4">
      {error && (
        <div className="error-banner" role="alert">
          <span className="error-banner-message">{error}</span>
          <Button variant="secondary" size="sm" onClick={() => void load()}>
            Retry
          </Button>
        </div>
      )}

      {/* Upstream DNS */}
      <div className="card">
        <div className="flex items-center gap-3 mb-4">
          <Globe size={20} className="text-text-secondary" />
          <div className="font-medium text-text-primary">Upstream DNS</div>
        </div>

        <div className="space-y-3">
          <div>
            <label htmlFor="dns-upstream-type" className="text-sm text-text-muted block mb-1">
              Type
            </label>
            <select
              id="dns-upstream-type"
              value={upstreamType}
              onChange={(e) => setUpstreamType(e.target.value as DnsUpstreamType)}
              className="w-full"
            >
              <option value="plainudp">Plain UDP (e.g., 8.8.8.8:53)</option>
              <option value="doh">DNS over HTTPS (DoH)</option>
            </select>
          </div>

          <div>
            <label htmlFor="dns-upstream-address" className="text-sm text-text-muted block mb-1">
              {upstreamType === "doh" ? "DoH URL" : "DNS Server"}
            </label>
            <input
              id="dns-upstream-address"
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
              disabled={loading || saving || !hasChanges}
              onClick={() => void save()}
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
            disabled={loading || saving || reloading}
            onClick={() => void reloadLists()}
          >
            {reloading ? "Reloading..." : "Reload"}
          </Button>
        </div>
      </div>
    </div>
  );
}

function errorMessage(context: string, cause: unknown): string {
  const detail = cause instanceof Error ? cause.message : String(cause);
  return `${context}: ${detail}`;
}
