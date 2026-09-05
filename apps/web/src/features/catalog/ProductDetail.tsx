"use client";

import { useCallback, useState } from "react";
import { ErrorState, LoadingState } from "@/components/FeedbackStates";
import { apiClient } from "@/lib/api-client";
import { resolveCartId } from "@/lib/cartStorage";
import { formatMoney } from "@/lib/money";
import {
  digitalDeliveryMethodLabel,
  fulfillmentTypeLabel,
  stockHint,
} from "@/lib/statusMaps";
import type { ProductDetail as ProductDetailData } from "@/lib/types";
import { useRemoteData } from "@/lib/useRemoteData";

export type ProductDetailProps = {
  productId: string;
};

export function ProductDetail({ productId }: ProductDetailProps) {
  const loader = useCallback(
    () => apiClient.get<ProductDetailData>(`/products/${productId}`),
    [productId],
  );
  const { data: product, error, isLoading, reload } = useRemoteData(
    loader,
    productId,
  );
  const [cartMessage, setCartMessage] = useState<string | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  const [actionError, setActionError] = useState<unknown>(null);

  async function handleAddToCart() {
    if (!product) {
      return;
    }
    const variant = product.variants[0];
    if (!variant || product.available_stock <= 0) {
      setCartMessage("This listing cannot be added until stock is available.");
      return;
    }

    setIsAdding(true);
    setActionError(null);
    setCartMessage(null);
    try {
      const cartId = resolveCartId();
      if (!cartId) {
        setActionError(new Error("No cart is available for this session."));
        return;
      }

      await apiClient.post(`/carts/${cartId}/items`, {
        product_id: product.product_id,
        variant_id: variant.variant_id,
        quantity: 1,
        source: "user",
      });
      setCartMessage(
        "Added to cart. Checkout uses the same checks for you and AI.",
      );
    } catch (addError) {
      setActionError(addError);
    } finally {
      setIsAdding(false);
    }
  }

  if (isLoading) {
    return <LoadingState label="Loading listing" />;
  }
  if (error && !product) {
    return <ErrorState error={error} onRetry={reload} />;
  }
  if (!product) {
    return null;
  }

  return (
    <article className="stack-lg">
      <header className="section-heading">
        <p className="meta">{product.store_name}</p>
        <h1 className="display">{product.title}</h1>
        <p className="money">
          {formatMoney(product.price_minor, product.currency)}
        </p>
      </header>

      <p className="lede">{product.description}</p>

      <dl className="fact-list">
        <div>
          <dt>Seller</dt>
          <dd>{product.store_name}</dd>
        </div>
        <div>
          <dt>Stock</dt>
          <dd>
            {stockHint(product.available_stock)}, {product.available_stock}{" "}
            available
          </dd>
        </div>
        <div>
          <dt>Delivery</dt>
          <dd>
            {fulfillmentTypeLabel(product.fulfillment_type)}
            {product.delivery_rules.digital_method
              ? `, ${digitalDeliveryMethodLabel(product.delivery_rules.digital_method)}`
              : ""}
            {product.delivery_rules.shipping_regions?.length
              ? `, ships to ${product.delivery_rules.shipping_regions.join(", ")}`
              : ""}
            , {product.delivery_rules.estimated_days_min}-
            {product.delivery_rules.estimated_days_max} days
          </dd>
        </div>
        <div>
          <dt>Refunds</dt>
          <dd>
            {product.refund_rules.is_refundable
              ? `${product.refund_rules.refund_window_days} day window. ${product.refund_rules.summary}`
              : product.refund_rules.summary}
          </dd>
        </div>
      </dl>

      <button
        type="button"
        className="button-primary"
        onClick={() => void handleAddToCart()}
        disabled={isAdding || product.available_stock <= 0}
      >
        {product.available_stock <= 0 ? "Out of stock" : "Add to cart"}
      </button>
      {cartMessage ? <p role="status">{cartMessage}</p> : null}
      {actionError ? <ErrorState error={actionError} /> : null}
    </article>
  );
}
