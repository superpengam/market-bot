import type { ReactNode } from "react";
import { ApiRequestError } from "@/lib/api-client";
import { errorCodeLabel } from "@/lib/statusMaps";

export function LoadingState({ label }: { label: string }) {
  return (
    <div className="feedback" role="status" aria-live="polite">
      <p className="feedback-title">{label}</p>
      <div className="skeleton-stack" aria-hidden="true">
        <span className="skeleton-rule" />
        <span className="skeleton-rule" />
        <span className="skeleton-rule short" />
      </div>
    </div>
  );
}

export function EmptyState({
  title,
  body,
  action,
}: {
  title: string;
  body: string;
  action?: ReactNode;
}) {
  return (
    <div className="feedback">
      <h2 className="display">{title}</h2>
      <p className="lede">{body}</p>
      {action}
    </div>
  );
}

export function ErrorState({
  error,
  onRetry,
}: {
  error: unknown;
  onRetry?: () => void;
}) {
  const requestError = error instanceof ApiRequestError ? error : null;
  const fallbackMessage =
    error instanceof Error
      ? error.message
      : "The catalog service could not be reached.";

  return (
    <div className="feedback feedback-error" role="alert">
      <h2 className="display">Request did not complete</h2>
      <p className="lede">
        {requestError ? errorCodeLabel(requestError.code) : fallbackMessage}
      </p>
      {requestError ? (
        <p className="meta money">Request {requestError.requestId}</p>
      ) : null}
      {onRetry ? (
        <button type="button" className="button-primary" onClick={onRetry}>
          Try again
        </button>
      ) : null}
    </div>
  );
}
