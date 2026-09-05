export const CART_STORAGE_KEY = "market-bot.cart_id";

export function readStoredCartId(): string | null {
  if (typeof window === "undefined") {
    return null;
  }
  const value = window.localStorage.getItem(CART_STORAGE_KEY);
  return value && value.trim() ? value : null;
}

export function writeStoredCartId(cartId: string): void {
  window.localStorage.setItem(CART_STORAGE_KEY, cartId);
}

export function resolveCartId(): string | null {
  const stored = readStoredCartId();
  if (stored) {
    return stored;
  }

  // Why: OpenAPI has no POST /carts; first add uses a session or documented fixture id.
  const fromEnv = process.env.NEXT_PUBLIC_DEV_CART_ID?.trim();
  if (!fromEnv) {
    return null;
  }

  if (typeof window !== "undefined") {
    writeStoredCartId(fromEnv);
  }
  return fromEnv;
}
