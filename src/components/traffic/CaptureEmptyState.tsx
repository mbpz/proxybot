import { Link } from "react-router-dom";
import { ArrowRight, KeyRound, Radio, Smartphone } from "lucide-react";
import { useOptionalCaptureSession } from "../../features/capture-session/CaptureSession";

/**
 * First-success guidance for an empty Capture workspace. Mirrors the
 * supported journey: start capture -> device proxy -> trust CA -> verify.
 */
export function CaptureEmptyState() {
  const capture = useOptionalCaptureSession();
  const running = capture?.running ?? false;

  return (
    <div className="flex h-full items-start justify-center overflow-auto p-6">
      <div className="w-full max-w-2xl">
        <div className="mb-6 flex flex-col items-center text-center">
          <Radio size={40} className="mb-3 text-accent-blue opacity-70" aria-hidden="true" />
          <h2 className="text-xl font-semibold text-text-primary">No requests captured yet</h2>
          <p className="mt-2 text-sm text-text-secondary">
            Follow these four steps to see your first decrypted request.
          </p>
        </div>

        <ol className="space-y-3">
          <li className="card flex items-center gap-4">
            <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-surface-tertiary font-semibold text-accent-blue">
              1
            </span>
            <div className="min-w-0 flex-1">
              <div className="font-medium text-text-primary">Start the Capture Session</div>
              <div className="text-sm text-text-muted">
                {running
                  ? "Capture is running."
                  : "Use the Start Capture button in the Capture bar above."}
              </div>
            </div>
            {running && (
              <span className="shrink-0 text-sm text-accent-green">Capturing</span>
            )}
          </li>

          <li className="card flex items-center gap-4">
            <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-surface-tertiary font-semibold text-accent-blue">
              2
            </span>
            <div className="min-w-0 flex-1">
              <div className="font-medium text-text-primary">Connect the test device</div>
              <div className="text-sm text-text-muted">
                Set the phone Wi-Fi proxy to this Mac's LAN IP and proxy port.
              </div>
            </div>
            <Link to="/setup" className="btn btn-secondary btn-sm shrink-0">
              <Smartphone size={14} /> Open Setup
            </Link>
          </li>

          <li className="card flex items-center gap-4">
            <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-surface-tertiary font-semibold text-accent-blue">
              3
            </span>
            <div className="min-w-0 flex-1">
              <div className="font-medium text-text-primary">Install and trust the CA</div>
              <div className="text-sm text-text-muted">
                Scan the Setup QR code, then enable full trust on iOS or install the certificate
                on Android.
              </div>
            </div>
            <Link to="/setup" className="btn btn-secondary btn-sm shrink-0">
              <KeyRound size={14} /> CA steps
            </Link>
          </li>

          <li className="card flex items-center gap-4">
            <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-surface-tertiary font-semibold text-accent-blue">
              4
            </span>
            <div className="min-w-0 flex-1">
              <div className="font-medium text-text-primary">Verify a request</div>
              <div className="text-sm text-text-muted">
                Open http://example.com then https://example.com on the device. The HTTPS request
                appears here once decryption works.
              </div>
            </div>
            <ArrowRight size={16} className="shrink-0 text-text-muted" aria-hidden="true" />
          </li>
        </ol>

        <p className="mt-4 text-center text-xs text-text-muted">
          Cmd+K opens the command palette for direct navigation.
        </p>
      </div>
    </div>
  );
}
