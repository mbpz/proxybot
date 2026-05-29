import { useState } from "react";
import { useNetwork } from "../../hooks/useNetwork";
import { Button } from "../ui/Button";
import { Globe, Wifi, Shield } from "lucide-react";

export function NetworkTab() {
  const {
    networkInfo,
    pfEnabled,
    pfLoading,
    tunEnabled,
    tunLoading,
    enablePf,
    disablePf,
    enableTun,
  } = useNetwork();
  const [error, setError] = useState("");

  return (
    <div className="space-y-4">
      {error && (
        <div className="error-banner">
          <span className="error-banner-message">{error}</span>
          <Button variant="secondary" size="sm" onClick={() => setError("")}>
            Dismiss
          </Button>
        </div>
      )}

      {/* LAN IP */}
      <div className="card">
        <div className="flex items-center gap-3">
          <Globe size={20} className="text-text-secondary" />
          <div>
            <div className="font-medium text-text-primary">LAN IP</div>
            <div className="text-sm text-text-muted">
              {networkInfo
                ? `${networkInfo.lan_ip} (${networkInfo.interface})`
                : "Detecting..."}
            </div>
          </div>
        </div>
      </div>

      {/* Packet Filter */}
      <div className="card">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Shield size={20} className={pfEnabled ? "text-accent-green" : "text-text-secondary"} />
            <div>
              <div className="font-medium text-text-primary">
                Packet Filter (pf)
              </div>
              <div className="text-sm text-text-muted">
                Transparent proxy via macOS pf — redirects port 80/443 traffic
              </div>
              <span className={`badge mt-1 ${pfEnabled ? "badge-direct" : "badge-unknown"}`}>
                {pfEnabled ? "Enabled" : "Disabled"}
              </span>
            </div>
          </div>
          <Button
            variant={pfEnabled ? "danger" : "primary"}
            size="sm"
            disabled={pfLoading}
            onClick={() => pfEnabled ? disablePf(setError) : enablePf(setError)}
          >
            {pfLoading ? "..." : pfEnabled ? "Disable" : "Enable"}
          </Button>
        </div>
      </div>

      {/* TUN Mode */}
      <div className="card">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Wifi size={20} className={tunEnabled ? "text-accent-green" : "text-text-secondary"} />
            <div>
              <div className="font-medium text-text-primary">TUN Mode</div>
              <div className="text-sm text-text-muted">
                Virtual network interface — captures all system traffic without pf
              </div>
              <span className={`badge mt-1 ${tunEnabled ? "badge-direct" : "badge-unknown"}`}>
                {tunEnabled ? "Enabled" : "Disabled"}
              </span>
            </div>
          </div>
          {!tunEnabled && (
            <Button
              variant="primary"
              size="sm"
              disabled={tunLoading}
              onClick={() => enableTun(setError)}
            >
              {tunLoading ? "..." : "Enable"}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
