import { UpdateSettings } from "../setup/UpdateSettings";
import { Info } from "lucide-react";

export function AboutTab() {
  return (
    <div className="space-y-4">
      {/* App Info */}
      <div className="card">
        <div className="flex items-center gap-3 mb-3">
          <Info size={20} className="text-accent-blue" />
          <div className="font-medium text-text-primary">ProxyBot</div>
        </div>
        <div className="text-sm text-text-muted space-y-1">
          <p>A macOS desktop proxy tool for developers.</p>
          <p>Capture and decrypt HTTPS/WSS traffic from your phone on the same LAN.</p>
        </div>
      </div>

      {/* Update Check */}
      <div className="card">
        <UpdateSettings />
      </div>
    </div>
  );
}
