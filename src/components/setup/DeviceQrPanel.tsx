import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Platform = "ios" | "android";

/**
 * QR panel for one-tap mobile device onboarding.
 * Renders two tabs (iOS / Android), each showing a QR code that encodes
 * the LAN URL of the appropriate CertServer endpoint.
 */
export function DeviceQrPanel() {
  const [platform, setPlatform] = useState<Platform>("ios");
  const [svg, setSvg] = useState<string>("");
  const [error, setError] = useState<string>("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError("");
    invoke<string>("generate_device_qr", { platform })
      .then((result) => {
        if (!cancelled) {
          setSvg(result);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(String(err));
          setSvg("");
          setLoading(false);
        }
      });
    return () => { cancelled = true; };
  }, [platform]);

  return (
    <div className="card mb-6">
      <h3 className="text-lg font-semibold mb-2">Add Mobile Device</h3>
      <p className="text-sm text-text-muted mb-3">
        Scan with your phone to install WiFi proxy, DNS, and the ProxyBot
        CA in one tap. <strong>Make sure your phone is connected to the
        ProxyBot WiFi network before scanning.</strong>
      </p>

      <div className="flex gap-2 mb-4" role="tablist">
        {(["ios", "android"] as const).map((p) => (
          <button
            key={p}
            role="tab"
            aria-selected={platform === p}
            onClick={() => setPlatform(p)}
            className={`px-3 py-1.5 rounded ${
              platform === p ? "btn btn-primary btn-sm" : "btn btn-sm bg-surface-elevated"
            }`}
          >
            {p === "ios" ? "iOS" : "Android"}
          </button>
        ))}
      </div>

      {loading && <div className="text-sm text-text-muted">Loading...</div>}

      {error && (
        <div className="text-sm text-accent-red border border-accent-red rounded p-2">
          {error}
        </div>
      )}

      {svg && !error && (
        <div
          className="flex justify-center"
          dangerouslySetInnerHTML={{ __html: svg }}
          data-testid="device-qr-svg"
        />
      )}

      {platform === "ios" && !error && (
        <details className="mt-3 text-sm">
          <summary className="cursor-pointer text-text-muted">
            After installing the profile
          </summary>
          <p className="mt-2">
            iOS does not auto-trust user-installed CAs. Go to{" "}
            <strong>Settings → General → About → Certificate Trust
            Settings</strong> and enable <em>ProxyBot CA</em> full trust
            for HTTPS interception to work.
          </p>
        </details>
      )}
    </div>
  );
}
