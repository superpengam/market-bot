"use client";

import { useCallback } from "react";
import { useParams } from "next/navigation";
import { EmptyState, ErrorState, LoadingState } from "@/components/FeedbackStates";
import { OrderStatus } from "@/features/orders/OrderStatus";
import { apiClient } from "@/lib/api-client";
import type { OrderDetail } from "@/lib/types";
import { useRemoteData } from "@/lib/useRemoteData";

export default function OrderPage() {
  const params = useParams<{ orderId: string }>();
  const orderId = params.orderId ?? "";
  const loader = useCallback(async () => {
    if (!orderId) {
      return null;
    }
    return apiClient.get<OrderDetail>(`/orders/${orderId}`);
  }, [orderId]);
  const { data: order, error, isLoading, reload } = useRemoteData(
    loader,
    orderId,
  );

  if (isLoading) {
    return <LoadingState label="Loading order" />;
  }
  if (error) {
    return <ErrorState error={error} onRetry={reload} />;
  }
  if (!order) {
    return (
      <EmptyState
        title="Order not found"
        body="Check the order id. Payment, fulfillment, and shipment stay on separate status fields."
      />
    );
  }

  return <OrderStatus order={order} />;
}
