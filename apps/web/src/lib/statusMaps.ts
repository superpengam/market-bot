import type {
  CartItemSource,
  DigitalDeliveryMethod,
  ErrorCode,
  FileSecurityCheckStatus,
  FulfillmentStatus,
  FulfillmentType,
  OrderStatus,
  PaymentProviderRedirectStatus,
  PaymentStatus,
  ProductStatus,
  ShipmentStatus,
  ShippingMethod,
} from "@/lib/types";

export const ORDER_STATUS_LABEL: Record<OrderStatus, string> = {
  draft: "Draft",
  pending_confirmation: "Pending confirmation",
  pending_payment: "Pending payment",
  payment_processing: "Payment processing",
  paid: "Paid",
  fulfillment_processing: "Fulfillment processing",
  shipped: "Shipped",
  delivered: "Delivered",
  completed: "Completed",
  cancellation_requested: "Cancellation requested",
  cancelled: "Cancelled",
  refund_processing: "Refund processing",
  refunded: "Refunded",
  dispute_processing: "Dispute processing",
};

export const PAYMENT_STATUS_LABEL: Record<PaymentStatus, string> = {
  created: "Created",
  processing: "Processing",
  succeeded: "Succeeded",
  failed: "Failed",
  refund_processing: "Refund processing",
  refunded: "Refunded",
};

export const FULFILLMENT_STATUS_LABEL: Record<FulfillmentStatus, string> = {
  pending: "Pending",
  processing: "Processing",
  delivered: "Delivered",
  failed: "Failed",
  cancelled: "Cancelled",
};

export const SHIPMENT_STATUS_LABEL: Record<ShipmentStatus, string> = {
  pending: "Pending",
  label_created: "Label created",
  in_transit: "In transit",
  out_for_delivery: "Out for delivery",
  delivered: "Delivered",
  exception: "Exception",
  returned: "Returned",
};

export const PAYMENT_PROVIDER_STATUS_LABEL: Record<
  PaymentProviderRedirectStatus,
  string
> = {
  not_started: "Not started",
  redirecting: "Redirecting to payment provider",
  awaiting_return: "Waiting for payment provider",
  completed: "Payment provider returned",
  failed: "Payment provider failed",
};

export const FULFILLMENT_TYPE_LABEL: Record<FulfillmentType, string> = {
  digital: "Digital delivery",
  physical_standard: "Physical shipment",
};

export const CART_ITEM_SOURCE_LABEL: Record<CartItemSource, string> = {
  user: "Added by you",
  ai: "Added by AI",
};

export const DIGITAL_DELIVERY_METHOD_LABEL: Record<DigitalDeliveryMethod, string> =
  {
    file_download: "File download",
    license_key: "License key",
    redemption_code: "Redemption code",
    access_link: "Access link",
  };

export const SHIPPING_METHOD_LABEL: Record<ShippingMethod, string> = {
  standard_ground: "Standard ground",
  express: "Express",
  local_pickup: "Local pickup",
};

export const PRODUCT_STATUS_LABEL: Record<ProductStatus, string> = {
  draft: "Draft",
  pending_review: "Pending review",
  published: "Published",
  suspended: "Suspended",
  archived: "Archived",
};

export const FILE_SECURITY_CHECK_LABEL: Record<FileSecurityCheckStatus, string> =
  {
    pending: "Security scan pending",
    passed: "Security scan passed",
    failed: "Security scan failed",
    not_required: "Security scan not required",
  };

export const ERROR_CODE_LABEL: Record<ErrorCode, string> = {
  INVALID_INPUT: "The request could not be accepted",
  UNAUTHORIZED: "Sign in is required",
  FORBIDDEN: "This action is not allowed",
  NOT_FOUND: "The record was not found",
  PRODUCT_OUT_OF_STOCK: "This item is out of stock",
  PRICE_CHANGED: "The price changed and must be reconfirmed",
  PAYMENT_REQUIRES_ACTION: "The payment provider needs another step",
  AI_AUTHORIZATION_EXPIRED: "The AI purchase authorization expired",
  AUTO_PURCHASE_LIMIT_EXCEEDED: "The automatic purchase limit was exceeded",
  FULFILLMENT_FAILED: "Fulfillment failed",
  ORDER_STATE_INVALID: "The order cannot change to that state",
  IDEMPOTENCY_KEY_REUSED: "This write was already processed",
  INTERNAL_ERROR: "The service could not complete the request",
};

export function orderStatusLabel(status: OrderStatus): string {
  return ORDER_STATUS_LABEL[status];
}

export function paymentStatusLabel(status: PaymentStatus): string {
  return PAYMENT_STATUS_LABEL[status];
}

export function fulfillmentStatusLabel(status: FulfillmentStatus): string {
  return FULFILLMENT_STATUS_LABEL[status];
}

export function shipmentStatusLabel(status: ShipmentStatus): string {
  return SHIPMENT_STATUS_LABEL[status];
}

export function paymentProviderStatusLabel(
  status: PaymentProviderRedirectStatus,
): string {
  return PAYMENT_PROVIDER_STATUS_LABEL[status];
}

export function fulfillmentTypeLabel(type: FulfillmentType): string {
  return FULFILLMENT_TYPE_LABEL[type];
}

export function cartItemSourceLabel(source: CartItemSource): string {
  return CART_ITEM_SOURCE_LABEL[source];
}

export function digitalDeliveryMethodLabel(
  method: DigitalDeliveryMethod,
): string {
  return DIGITAL_DELIVERY_METHOD_LABEL[method];
}

export function shippingMethodLabel(method: string): string {
  if (method in SHIPPING_METHOD_LABEL) {
    return SHIPPING_METHOD_LABEL[method as ShippingMethod];
  }
  return SHIPPING_METHOD_LABEL.standard_ground;
}

export function errorCodeLabel(code: ErrorCode): string {
  return ERROR_CODE_LABEL[code];
}

export function stockHint(availableStock: number): string {
  if (availableStock <= 0) {
    return "Out of stock";
  }
  if (availableStock <= 5) {
    return "Low stock";
  }
  return "In stock";
}
