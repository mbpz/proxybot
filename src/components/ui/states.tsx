import type { ReactNode } from "react";
import { AlertCircle, Loader2, RefreshCw } from "lucide-react";
import { Button } from "./Button";

/** Shared async/empty/error presentation so pages stop inventing their own. */

export function LoadingState({ label = "Loading..." }: { label?: string }) {
  return (
    <div
      role="status"
      aria-live="polite"
      className="flex items-center justify-center gap-2 p-8 text-sm text-text-muted"
    >
      <Loader2 size={16} className="animate-spin" aria-hidden="true" />
      {label}
    </div>
  );
}

interface EmptyStateProps {
  icon?: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
}

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="empty-state">
      {icon && <div className="empty-state-icon">{icon}</div>}
      <div className="empty-state-title">{title}</div>
      {description && <div className="empty-state-description">{description}</div>}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}

interface ErrorStateProps {
  title?: string;
  message: string;
  onRetry?: () => void;
  retryLabel?: string;
}

export function ErrorState({
  title = "Something went wrong",
  message,
  onRetry,
  retryLabel = "Retry",
}: ErrorStateProps) {
  return (
    <div className="error-banner" role="alert">
      <AlertCircle size={16} aria-hidden="true" />
      <span className="error-banner-message">
        <span className="font-medium">{title}: </span>
        {message}
      </span>
      {onRetry && (
        <Button variant="secondary" size="sm" onClick={onRetry}>
          <RefreshCw size={14} /> {retryLabel}
        </Button>
      )}
    </div>
  );
}

type NoticeVariant = "info" | "success" | "error";

const NOTICE_CLASS: Record<NoticeVariant, string> = {
  info: "border-accent-blue bg-[rgba(77,157,224,0.1)] text-accent-blue",
  success: "border-accent-green bg-[rgba(62,207,142,0.1)] text-accent-green",
  error: "border-accent-red bg-[rgba(231,111,81,0.1)] text-accent-red",
};

interface InlineNoticeProps {
  variant?: NoticeVariant;
  children: ReactNode;
  onDismiss?: () => void;
}

export function InlineNotice({ variant = "info", children, onDismiss }: InlineNoticeProps) {
  return (
    <div
      role={variant === "error" ? "alert" : "status"}
      className={`mb-3 flex items-center gap-2 rounded-md border px-3 py-2 text-sm ${NOTICE_CLASS[variant]}`}
    >
      <span className="min-w-0 flex-1">{children}</span>
      {onDismiss && (
        <button
          type="button"
          onClick={onDismiss}
          aria-label="Dismiss notice"
          className="rounded p-0.5 hover:bg-white/10"
        >
          ×
        </button>
      )}
    </div>
  );
}
