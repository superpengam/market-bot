import { readAccessToken } from "@/lib/session";
import type { ApiErrorBody, ErrorCode } from "@/lib/types";

export type ApiRequestOptions = {
  query?: Record<string, string | number | boolean | undefined | null>;
  accessToken?: string;
  requestId?: string;
  idempotencyKey?: string;
  signal?: AbortSignal;
};

export type ApiClientConfig = {
  baseUrl?: string;
  getAccessToken?: () => string | undefined;
  generateRequestId?: () => string;
  generateIdempotencyKey?: () => string;
  fetchImpl?: typeof fetch;
  credentials?: RequestCredentials;
};

const ERROR_CODES = new Set<ErrorCode>([
  "INVALID_INPUT",
  "UNAUTHORIZED",
  "FORBIDDEN",
  "NOT_FOUND",
  "PRODUCT_OUT_OF_STOCK",
  "PRICE_CHANGED",
  "PAYMENT_REQUIRES_ACTION",
  "AI_AUTHORIZATION_EXPIRED",
  "AUTO_PURCHASE_LIMIT_EXCEEDED",
  "FULFILLMENT_FAILED",
  "ORDER_STATE_INVALID",
  "IDEMPOTENCY_KEY_REUSED",
  "INTERNAL_ERROR",
]);

export class ApiRequestError extends Error {
  readonly code: ErrorCode;
  readonly requestId: string;
  readonly status: number;

  constructor(
    code: ErrorCode,
    message: string,
    requestId: string,
    status: number,
  ) {
    super(message);
    this.name = "ApiRequestError";
    this.code = code;
    this.requestId = requestId;
    this.status = status;
  }
}

function isApiErrorBody(value: unknown): value is ApiErrorBody {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const body = value as Partial<ApiErrorBody>;
  return (
    typeof body.code === "string" &&
    ERROR_CODES.has(body.code) &&
    typeof body.message === "string" &&
    typeof body.request_id === "string"
  );
}

function createId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `req-${Date.now()}-fallback`;
}

function toQueryString(
  query: ApiRequestOptions["query"],
): string {
  if (!query) {
    return "";
  }

  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (value === undefined || value === null || value === "") {
      continue;
    }
    params.set(key, String(value));
  }

  const text = params.toString();
  return text ? `?${text}` : "";
}

export class ApiClient {
  private readonly baseUrl: string;
  private readonly getAccessToken?: () => string | undefined;
  private readonly generateRequestId: () => string;
  private readonly generateIdempotencyKey: () => string;
  private readonly fetchImpl: typeof fetch;
  private readonly credentials: RequestCredentials;

  constructor(config: ApiClientConfig = {}) {
    this.baseUrl = (config.baseUrl ??
      process.env.NEXT_PUBLIC_API_BASE_URL ??
      "/api/v1"
    ).replace(/\/$/, "");
    this.getAccessToken = config.getAccessToken;
    this.generateRequestId = config.generateRequestId ?? createId;
    this.generateIdempotencyKey = config.generateIdempotencyKey ?? createId;
    this.fetchImpl = config.fetchImpl ?? fetch.bind(globalThis);
    this.credentials = config.credentials ?? "include";
  }

  get<T>(path: string, options: ApiRequestOptions = {}): Promise<T> {
    return this.request<T>("GET", path, undefined, options);
  }

  post<T>(
    path: string,
    body: unknown,
    options: ApiRequestOptions = {},
  ): Promise<T> {
    return this.request<T>("POST", path, body, options);
  }

  patch<T>(
    path: string,
    body: unknown,
    options: ApiRequestOptions = {},
  ): Promise<T> {
    return this.request<T>("PATCH", path, body, options);
  }

  delete<T>(path: string, options: ApiRequestOptions = {}): Promise<T> {
    return this.request<T>("DELETE", path, undefined, options);
  }

  private async request<T>(
    method: string,
    path: string,
    body: unknown,
    options: ApiRequestOptions,
  ): Promise<T> {
    const requestId = options.requestId ?? this.generateRequestId();
    const normalizedPath = path.startsWith("/") ? path : `/${path}`;
    const url = `${this.baseUrl}${normalizedPath}${toQueryString(options.query)}`;
    const headers = new Headers();
    headers.set("Accept", "application/json");
    headers.set("X-Request-Id", requestId);

    const accessToken = options.accessToken ?? this.getAccessToken?.();
    if (accessToken) {
      headers.set("Authorization", `Bearer ${accessToken}`);
    }

    const isWrite = method !== "GET";
    if (isWrite) {
      // Why: write operations must be replay-safe across retries and AI clients.
      headers.set(
        "Idempotency-Key",
        options.idempotencyKey ?? this.generateIdempotencyKey(),
      );
    }

    let serializedBody: string | undefined;
    if (body !== undefined && method !== "GET") {
      headers.set("Content-Type", "application/json");
      serializedBody = JSON.stringify(body);
    }

    const response = await this.fetchImpl(url, {
      method,
      headers,
      body: serializedBody,
      signal: options.signal,
      credentials: this.credentials,
    });

    if (response.status === 204) {
      return undefined as T;
    }

    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      if (isApiErrorBody(payload)) {
        throw new ApiRequestError(
          payload.code,
          payload.message,
          payload.request_id,
          response.status,
        );
      }
      throw new ApiRequestError(
        "INTERNAL_ERROR",
        "Request failed",
        requestId,
        response.status,
      );
    }

    return payload as T;
  }
}

export function createBrowserApiClient(
  config: ApiClientConfig = {},
): ApiClient {
  return new ApiClient({
    ...config,
    getAccessToken: config.getAccessToken ?? readAccessToken,
    credentials: config.credentials ?? "include",
  });
}

export const apiClient = createBrowserApiClient();
