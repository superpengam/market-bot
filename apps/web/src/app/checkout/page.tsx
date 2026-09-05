"use client";

import Link from "next/link";
import { useCallback, useState } from "react";
import { EmptyState, ErrorState, LoadingState } from "@/components/FeedbackStates";
import { CheckoutPreview } from "@/features/checkout/CheckoutPreview";
import { apiClient } from "@/lib/api-client";
import { resolveCartId } from "@/lib/cartStorage";
import { followPaymentRedirect, placeOrderIdempotencyKey } from "@/lib/orderWrite";
import type { CheckoutPreview as CheckoutPreviewData, OrderDetail } from "@/lib/types";
import { useRemoteData } from "@/lib/useRemoteData";

export default function CheckoutPage() {
  const loader = useCallback(async () => {
    const cartId = resolveCartId();
    if (!cartId) {
      return null;
    }
    return apiClient.post<CheckoutPreviewData>("/checkout/preview", {
      cart_id: cartId,
    });
  }, []);
  const { data: preview, error, isLoading, reload } = useRemoteData(
    loader,
    "checkout-preview",
  );
  const [order, setOrder] = useState<OrderDetail | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<unknown>(null);

  async function handlePlaceOrder() {
    const cartId = resolveCartId();
    if (!cartId || !preview || preview.requires_price_reconfirm) {
      return;
    }
    if (!preview.inventory_is_available) {
      return;
    }

    setIsSubmitting(true);
    setSubmitError(null);
    try {
      const created = await apiClient.post<OrderDetail>(
        "/orders",
        { cart_id: cartId },
        {
          idempotencyKey: placeOrderIdempotencyKey(cartId, preview.expires_at),
        },
      );
      if (followPaymentRedirect(created)) {
        return;
      }
      setOrder(created);
    } catch (submitErr) {
      setSubmitError(submitErr);
    } finally {
      setIsSubmitting(false);
    }
  }

  if (isLoading) {
    return <LoadingState label="Preparing checkout" />;
  }
  if (!preview && !error) {
    return (
      <EmptyState
        title="Nothing to check out"
        body="Add items to the cart first. AI-added items use the same preview."
        action={
          <Link className="button-primary" href="/cart">
            Open cart
          </Link>
        }
      />
    );
  }
  if (error && !preview) {
    return <ErrorState error={error} onRetry={reload} />;
  }
  if (!preview) {
    return null;
  }

  const canPay =
    !preview.requires_price_reconfirm &&
    preview.inventory_is_available &&
    preview.payment_provider_status !== "failed";

  return (
    <div className="stack-lg">
      <CheckoutPreview preview={preview} onReconfirm={reload} />
      {submitError ? <ErrorState error={submitError} /> : null}
      {order ? (
        <p role="status">
          Order created.{" "}
          <Link href={`/orders/${order.order_id}`}>View order status</Link>
        </p>
      ) : (
        <button
          type="button"
          className="button-primary"
          disabled={!canPay || isSubmitting}
          onClick={() => void handlePlaceOrder()}
        >
          Place order
        </button>
      )}
    </div>
  );
}
