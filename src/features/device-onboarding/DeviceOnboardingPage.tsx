import { useMemo, useState } from "react";
import DOMPurify from "dompurify";
import {
  AlertCircle,
  ArrowRight,
  CheckCircle2,
  ExternalLink,
  KeyRound,
  Play,
  Server,
  Smartphone,
  Square,
  Wifi,
} from "lucide-react";
import { Link } from "react-router-dom";
import { Button } from "../../components/ui/Button";
import { desktop, type DesktopContract } from "../../desktop/contract";
import type { DeviceOnboarding, DevicePlatform } from "../../generated/desktop-contract";
import { useCaptureSession } from "../capture-session/CaptureSession";

interface DeviceOnboardingPageProps {
  contract?: DesktopContract;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function DeviceOnboardingPage({ contract = desktop }: DeviceOnboardingPageProps) {
  const capture = useCaptureSession();
  const [platform, setPlatform] = useState<DevicePlatform>("ios");
  const [onboarding, setOnboarding] = useState<DeviceOnboarding | null>(null);
  const [preparing, setPreparing] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const prepared = onboarding?.platform === platform ? onboarding : null;
  const sanitizedQr = useMemo(
    () => (prepared ? DOMPurify.sanitize(prepared.qr_svg, { USE_PROFILES: { svg: true } }) : ""),
    [prepared],
  );

  async function prepare() {
    setPreparing(true);
    setError(null);
    try {
      setOnboarding(await contract.call("prepare_device_onboarding", { platform }));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setPreparing(false);
    }
  }

  async function stopSetupServer() {
    setStopping(true);
    setError(null);
    try {
      await contract.call("stop_device_onboarding", {});
      setOnboarding(null);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setStopping(false);
    }
  }

  const captureBusy = capture.operation !== null;
  const platformLabel = platform === "ios" ? "iOS" : "Android";

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <header className="space-y-2">
        <div className="flex items-center gap-3">
          <Smartphone size={26} className="text-accent-blue" />
          <h1 className="text-2xl font-bold">Device Setup</h1>
        </div>
        <p className="max-w-3xl text-text-secondary">
          Connect one iOS or Android test device with an explicit Wi-Fi proxy. ProxyBot keeps
          capture and the temporary certificate server separate so each can be stopped safely.
        </p>
      </header>

      {error && (
        <div className="error-banner" role="alert">
          <AlertCircle size={17} />
          <span className="error-banner-message">{error}</span>
          <Button variant="ghost" size="sm" onClick={() => void prepare()} disabled={preparing}>
            Retry
          </Button>
        </div>
      )}

      <section className="card space-y-4" aria-labelledby="platform-heading">
        <div>
          <h2 id="platform-heading" className="text-lg font-semibold">Choose a device</h2>
          <p className="mt-1 text-sm text-text-muted">
            The Mac and phone must be on the same trusted Wi-Fi network.
          </p>
        </div>
        <div className="flex gap-2" role="tablist" aria-label="Mobile platform">
          {(["ios", "android"] as const).map((value) => (
            <button
              key={value}
              type="button"
              role="tab"
              aria-selected={platform === value}
              className={"btn " + (platform === value ? "btn-primary" : "btn-secondary")}
              onClick={() => setPlatform(value)}
            >
              {value === "ios" ? "iOS" : "Android"}
            </button>
          ))}
        </div>
      </section>

      <div className="grid gap-4 lg:grid-cols-2">
        <SetupStep
          number="1"
          title="Start the Capture Session"
          icon={<Play size={19} />}
          complete={capture.running}
        >
          <p className="text-sm text-text-secondary">
            The capture listener must be running before the device sends traffic.
          </p>
          <div className="mt-4">
            {capture.running ? (
              <span className="inline-flex items-center gap-2 text-sm text-accent-green">
                <CheckCircle2 size={16} /> Capture is running
              </span>
            ) : (
              <Button
                variant="primary"
                onClick={() => void capture.start()}
                disabled={captureBusy}
              >
                <Play size={16} />
                {capture.operation === "starting" ? "Starting..." : "Start Capture"}
              </Button>
            )}
          </div>
        </SetupStep>

        <SetupStep
          number="2"
          title="Prepare this Mac"
          icon={<Server size={19} />}
          complete={Boolean(prepared)}
        >
          <p className="text-sm text-text-secondary">
            Discover the active LAN address and start a temporary CA download server.
          </p>
          <div className="mt-4 flex flex-wrap gap-2">
            <Button variant="primary" onClick={() => void prepare()} disabled={preparing}>
              <Wifi size={16} />
              {preparing
                ? "Preparing..."
                : (prepared ? "Refresh " : "Prepare ") + platformLabel + " Setup"}
            </Button>
            {onboarding && (
              <Button
                variant="danger"
                onClick={() => void stopSetupServer()}
                disabled={stopping}
              >
                <Square size={15} />
                {stopping ? "Stopping..." : "Stop Setup Server"}
              </Button>
            )}
          </div>
        </SetupStep>
      </div>

      {prepared ? (
        <div className="grid gap-4 lg:grid-cols-2">
          <SetupStep number="3" title="Configure the device" icon={<Wifi size={19} />}>
            <p className="text-sm text-text-secondary">
              Open the connected Wi-Fi network, choose a manual proxy, then enter:
            </p>
            <dl className="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-3 rounded-lg border border-border bg-surface-secondary p-4 text-sm">
              <dt className="text-text-muted">Server</dt>
              <dd className="select-all font-mono text-text-primary">{prepared.lan_ip}</dd>
              <dt className="text-text-muted">Port</dt>
              <dd className="select-all font-mono text-text-primary">{prepared.proxy_port}</dd>
              <dt className="text-text-muted">Interface</dt>
              <dd className="font-mono text-text-secondary">{prepared.interface}</dd>
            </dl>
            <p className="mt-3 text-xs text-text-muted">
              {platform === "ios"
                ? "iOS: Settings → Wi-Fi → network details → Configure Proxy → Manual."
                : "Android: Wi-Fi network details → Edit → Advanced options → Proxy → Manual."}
            </p>
          </SetupStep>

          <SetupStep number="4" title="Install and verify the CA" icon={<KeyRound size={19} />}>
            <div className="grid gap-4 sm:grid-cols-[160px_1fr]">
              <div
                className="flex min-h-40 items-center justify-center rounded-lg bg-white p-2 text-black"
                dangerouslySetInnerHTML={{ __html: sanitizedQr }}
                data-testid="device-onboarding-qr"
              />
              <div className="space-y-3 text-sm text-text-secondary">
                <p>
                  Scan the QR code on the device.
                  {platform === "ios"
                    ? " It downloads only the ProxyBot CA certificate."
                    : " It opens a local setup page with the CA download."}
                </p>
                {platform === "ios" ? (
                  <p>
                    Install the CA, then enable full trust in Settings → General → About →
                    Certificate Trust Settings.
                  </p>
                ) : (
                  <p>
                    User-installed CAs are not trusted by every Android app. App developers may
                    need a Network Security Configuration for debug builds.
                  </p>
                )}
                <a
                  href={prepared.setup_url}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1"
                >
                  Open setup URL <ExternalLink size={13} />
                </a>
              </div>
            </div>
            <div className="mt-4 rounded-lg border border-border bg-surface-secondary p-4 text-sm">
              <p>
                Verify in order: open <code>http://example.com</code>, then{" "}
                <code>https://example.com</code> on the device.
              </p>
              <Link to="/" className="mt-3 inline-flex items-center gap-1">
                Check captured requests in Traffic <ArrowRight size={14} />
              </Link>
            </div>
          </SetupStep>
        </div>
      ) : (
        <div className="rounded-lg border border-dashed border-border-light p-8 text-center text-sm text-text-muted">
          Prepare this Mac to reveal the exact proxy address, QR code, and verification steps.
        </div>
      )}

      <footer className="flex flex-wrap gap-x-5 gap-y-2 border-t border-border pt-4 text-sm">
        <Link to="/certs">Advanced certificate and decryption controls</Link>
        <Link to="/devices">Registered device inventory</Link>
      </footer>
    </div>
  );
}

interface SetupStepProps {
  number: string;
  title: string;
  icon: React.ReactNode;
  complete?: boolean;
  children: React.ReactNode;
}

function SetupStep({ number, title, icon, complete = false, children }: SetupStepProps) {
  return (
    <section className="card h-full">
      <div className="mb-4 flex items-center gap-3">
        <span
          className={
            "flex h-8 w-8 items-center justify-center rounded-full text-sm font-semibold " +
            (complete
              ? "bg-accent-green text-black"
              : "bg-surface-tertiary text-accent-blue")
          }
        >
          {complete ? <CheckCircle2 size={17} aria-label="Complete" /> : number}
        </span>
        <span className="text-accent-blue">{icon}</span>
        <h2 className="text-lg font-semibold">{title}</h2>
      </div>
      {children}
    </section>
  );
}
