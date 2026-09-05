import { ApiClient } from "@/lib/api-client";

const ACCESS_TOKEN_STORAGE_KEY = "market-bot.access_token";

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

test("should_send_bearer_token_and_include_credentials_on_every_request", async () => {
  const fetchImpl = jest.fn().mockResolvedValue(jsonResponse({ items: [] }));
  const client = new ApiClient({
    baseUrl: "/api/v1",
    getAccessToken: () => "session-token",
    fetchImpl: fetchImpl as unknown as typeof fetch,
  });

  await client.get("/products/search");

  expect(fetchImpl).toHaveBeenCalledTimes(1);
  const [, init] = fetchImpl.mock.calls[0] as [string, RequestInit];
  expect(init.credentials).toBe("include");
  expect(new Headers(init.headers).get("Authorization")).toBe(
    "Bearer session-token",
  );
});

test("should_reuse_caller_supplied_idempotency_key_on_writes", async () => {
  const fetchImpl = jest.fn().mockResolvedValue(jsonResponse({ order_id: "o1" }));
  const client = new ApiClient({
    baseUrl: "/api/v1",
    fetchImpl: fetchImpl as unknown as typeof fetch,
  });

  await client.post("/orders", { cart_id: "c1" }, { idempotencyKey: "stable-key" });
  await client.post("/orders", { cart_id: "c1" }, { idempotencyKey: "stable-key" });

  const first = new Headers(
    (fetchImpl.mock.calls[0] as [string, RequestInit])[1].headers,
  ).get("Idempotency-Key");
  const second = new Headers(
    (fetchImpl.mock.calls[1] as [string, RequestInit])[1].headers,
  ).get("Idempotency-Key");
  expect(first).toBe("stable-key");
  expect(second).toBe("stable-key");
});

test("shared_client_reads_session_token_and_includes_credentials", async () => {
  window.sessionStorage.setItem(ACCESS_TOKEN_STORAGE_KEY, "stored-session");
  const fetchImpl = jest.fn().mockResolvedValue(jsonResponse({ ok: true }));
  const { createBrowserApiClient } = jest.requireActual("./api-client") as {
    createBrowserApiClient: (config: {
      baseUrl?: string;
      fetchImpl?: typeof fetch;
    }) => ApiClient;
  };
  const client = createBrowserApiClient({
    baseUrl: "/api/v1",
    fetchImpl: fetchImpl as unknown as typeof fetch,
  });

  await client.post("/seller/products", { title: "Notes" });

  const [, init] = fetchImpl.mock.calls[0] as [string, RequestInit];
  expect(init.credentials).toBe("include");
  expect(new Headers(init.headers).get("Authorization")).toBe(
    "Bearer stored-session",
  );
  window.sessionStorage.removeItem(ACCESS_TOKEN_STORAGE_KEY);
});
