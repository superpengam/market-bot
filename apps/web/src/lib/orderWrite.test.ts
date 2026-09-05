import {
  followPaymentRedirect,
  paymentRedirectUrl,
  placeOrderIdempotencyKey,
} from "@/lib/orderWrite";
import type { OrderDetail } from "@/lib/types";

function order(overrides: Partial<OrderDetail> = {}): OrderDetail {
  return {
    order_id: "ord-1",
    order_status: "pending_payment",
    payment_status: "created",
    fulfillment_status: "pending",
    shipment_status: null,
    items: [],
    subtotal_minor: 100,
    shipping_fee_minor: 0,
    tax_minor: 0,
    total_minor: 100,
    currency: "USD",
    created_at: "2026-09-03T14:00:00.000Z",
    ...overrides,
  };
}

afterEach(() => {
  window.sessionStorage.clear();
});

test("should_reuse_one_idempotency_key_per_cart_and_preview_expiry", () => {
  const first = placeOrderIdempotencyKey("cart-1", "2026-09-03T14:30:00.000Z");
  const retry = placeOrderIdempotencyKey("cart-1", "2026-09-03T14:30:00.000Z");
  const nextPreview = placeOrderIdempotencyKey(
    "cart-1",
    "2026-09-03T15:00:00.000Z",
  );

  expect(first).toEqual(retry);
  expect(nextPreview).not.toEqual(first);
});

test("should_read_provider_redirect_or_intent_url_from_order", () => {
  expect(
    paymentRedirectUrl(
      order({ payment_redirect_url: "https://pay.example/redirect" }),
    ),
  ).toBe("https://pay.example/redirect");
  expect(
    paymentRedirectUrl(order({ payment_intent_url: "https://pay.example/intent" })),
  ).toBe("https://pay.example/intent");
  expect(paymentRedirectUrl(order())).toBeNull();
});

test("should_follow_provider_url_and_skip_when_absent", () => {
  const assign = jest.fn();
  expect(followPaymentRedirect(order(), assign)).toBe(false);
  expect(assign).not.toHaveBeenCalled();

  expect(
    followPaymentRedirect(
      order({ redirect_url: "https://pay.example/checkout" }),
      assign,
    ),
  ).toBe(true);
  expect(assign).toHaveBeenCalledWith("https://pay.example/checkout");
});
