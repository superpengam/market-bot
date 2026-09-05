"use client";

import Link from "next/link";
import { useCallback } from "react";
import { EmptyState, ErrorState, LoadingState } from "@/components/FeedbackStates";
import { apiClient } from "@/lib/api-client";
import { resolveCartId } from "@/lib/cartStorage";
import { formatMoney } from "@/lib/money";
import {
  cartItemSourceLabel,
  fulfillmentTypeLabel,
  stockHint,
} from "@/lib/statusMaps";
import type { Cart, CartItem } from "@/lib/types";
import { useRemoteData } from "@/lib/useRemoteData";

function validateCheckoutItem(item: CartItem): string | null {
  if (item.quantity <= 0) {
    return "Quantity must be greater than zero.";
  }
  if (item.available_stock < item.quantity) {
    return "Quantity exceeds available stock.";
  }
  if (item.unit_price_minor < 0) {
    return "Price snapshot is invalid.";
  }
  return null;
}

export function CartPanel() {
  const loader = useCallback(async () => {
    const cartId = resolveCartId();
    if (!cartId) {
      return null;
    }
    return apiClient.get<Cart>(`/carts/${cartId}`);
  }, []);
  const { data: cart, error, isLoading, reload } = useRemoteData(loader, "cart");

  if (isLoading) {
    return <LoadingState label="Loading cart" />;
  }
  if (error) {
    return <ErrorState error={error} onRetry={reload} />;
  }
  if (!cart || cart.items.length === 0) {
    return (
      <EmptyState
        title="Cart is empty"
        body="Add a listing yourself or let an authorized AI add one. Both sources use the same checkout checks."
        action={
          <Link className="button-primary" href="/products">
            Browse catalog
          </Link>
        }
      />
    );
  }

  const blockingIssues = cart.items
    .map((item) => ({ item, issue: validateCheckoutItem(item) }))
    .filter((entry) => entry.issue);

  return (
    <section className="stack-lg">
      <header className="section-heading">
        <h1 className="display">Cart</h1>
        <p className="lede">
          Items added by you and by AI stay in one cart. Checkout still
          rechecks price, stock, shipping, and tax.
        </p>
      </header>

      <ol className="editorial-list">
        {cart.items.map((item) => {
          const issue = validateCheckoutItem(item);
          return (
            <li key={item.cart_item_id} className="editorial-row">
              <div className="editorial-main">
                <h2>
                  <Link href={`/products/${item.product_id}`}>{item.title}</Link>
                </h2>
                <p className="meta">
                  {cartItemSourceLabel(item.source)},{" "}
                  {fulfillmentTypeLabel(item.fulfillment_type)}, qty{" "}
                  {item.quantity}, {stockHint(item.available_stock)}
                </p>
                {issue ? (
                  <p className="banner banner-alert" role="alert">
                    {issue}
                  </p>
                ) : null}
              </div>
              <p className="money">
                {formatMoney(item.unit_price_minor, item.currency)}
              </p>
            </li>
          );
        })}
      </ol>

      {blockingIssues.length === 0 ? (
        <Link className="button-primary" href="/checkout">
          Review checkout
        </Link>
      ) : (
        <p role="alert">Fix stock or quantity before checkout.</p>
      )}
    </section>
  );
}
