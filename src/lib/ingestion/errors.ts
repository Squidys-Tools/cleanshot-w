export type UrlIngestionErrorCode =
  | "content-too-large"
  | "extraction-failed"
  | "invalid-url"
  | "network-error"
  | "parse-failed"
  | "unsupported-content-type"
  | "http-error"
  | "timeout";

export interface UrlIngestionErrorDetails {
  cause?: unknown;
  status?: number;
  url?: string;
}

export class UrlIngestionError extends Error {
  readonly name = "UrlIngestionError";

  constructor(
    readonly code: UrlIngestionErrorCode,
    message: string,
    readonly details: UrlIngestionErrorDetails = {},
  ) {
    super(message);
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

export function isUrlIngestionError(error: unknown): error is UrlIngestionError {
  return error instanceof UrlIngestionError;
}
