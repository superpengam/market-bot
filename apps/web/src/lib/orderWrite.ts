import type { OrderDetail } from "@/lib/types";

const PLACE_ORDER_KEY_PREFIX = "market-bot.place-order";

function createId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `idem-${Date.now()}-fallback`;
}

export function placeOrderIdempotencyKey(
  cartId: string,
  previewExpiresAt: string,
): string {
  const storageKey = `${PLACE_ORDER_KEY_PREFIX}:${cartId}:${previewExpiresAt}`;
  if (typeof window === "undefined") {
    return createId();
  }

  const existing = window.sessionStorage.getItem(storageKey);
  if (existing?.trim()) {
    return existing;
  }

  const next = createId();
  window.sessionStorage.setItem(storageKey, next);
  return next;
}

export function paymentRedirectUrl(order: OrderDetail): string | null {
  const url =
    order.payment_redirect_url ??
    order.payment_intent_url ??
    order.redirect_url ??
    null;
  return url && url.trim() ? url : null;
}

export function followPaymentRedirect(
  order: OrderDetail,
  assign: (url: string) => void = (url) => {
    window.location.assign(url);
  },
): boolean {
  const url = paymentRedirectUrl(order);
  if (!url) {
    return false;
  }
  assign(url);
  return true;
}
