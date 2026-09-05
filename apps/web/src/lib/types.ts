export type CurrencyCode = string;

export type FulfillmentType = "digital" | "physical_standard";

export type CartItemSource = "user" | "ai";

export type ProductStatus =
  | "draft"
  | "pending_review"
  | "published"
  | "suspended"
  | "archived";

export type OrderStatus =
  | "draft"
  | "pending_confirmation"
  | "pending_payment"
  | "payment_processing"
  | "paid"
  | "fulfillment_processing"
  | "shipped"
  | "delivered"
  | "completed"
  | "cancellation_requested"
  | "cancelled"
  | "refund_processing"
  | "refunded"
  | "dispute_processing";

export type PaymentStatus =
  | "created"
  | "processing"
  | "succeeded"
  | "failed"
  | "refund_processing"
  | "refunded";

export type FulfillmentStatus =
  | "pending"
  | "processing"
  | "delivered"
  | "failed"
  | "cancelled";

export type ShipmentStatus =
  | "pending"
  | "label_created"
  | "in_transit"
  | "out_for_delivery"
  | "delivered"
  | "exception"
  | "returned";

export type PaymentProviderRedirectStatus =
  | "not_started"
  | "redirecting"
  | "awaiting_return"
  | "completed"
  | "failed";

export type ErrorCode =
  | "INVALID_INPUT"
  | "UNAUTHORIZED"
  | "FORBIDDEN"
  | "NOT_FOUND"
  | "PRODUCT_OUT_OF_STOCK"
  | "PRICE_CHANGED"
  | "PAYMENT_REQUIRES_ACTION"
  | "AI_AUTHORIZATION_EXPIRED"
  | "AUTO_PURCHASE_LIMIT_EXCEEDED"
  | "FULFILLMENT_FAILED"
  | "ORDER_STATE_INVALID"
  | "IDEMPOTENCY_KEY_REUSED"
  | "INTERNAL_ERROR";

export type DigitalDeliveryMethod =
  | "file_download"
  | "license_key"
  | "redemption_code"
  | "access_link";

export type ShippingMethod = "standard_ground" | "express" | "local_pickup";

export type FileSecurityCheckStatus =
  | "pending"
  | "passed"
  | "failed"
  | "not_required";

export type MoneySnapshot = {
  amount_minor: number;
  currency: CurrencyCode;
};

export type ApiErrorBody = {
  code: ErrorCode;
  message: string;
  request_id: string;
};

export type ProductSearchResult = {
  product_id: string;
  variant_ids: string[];
  title: string;
  attributes?: Record<string, string>;
  price_minor: number;
  currency: CurrencyCode;
  available_stock: number;
  fulfillment_type: FulfillmentType;
};

export type ProductSearchPage = {
  items: ProductSearchResult[];
  next_cursor: string | null;
};

export type ProductSearchQuery = {
  q?: string;
  category_id?: string;
  currency?: CurrencyCode;
  min_price_minor?: number;
  max_price_minor?: number;
  fulfillment_type?: FulfillmentType;
  cursor?: string;
};

export type ProductVariant = {
  variant_id: string;
  sku: string;
  title?: string;
  price_minor: number;
  currency: CurrencyCode;
  available_stock: number;
};

export type DeliveryRules = {
  fulfillment_type: FulfillmentType;
  estimated_days_min: number;
  estimated_days_max: number;
  digital_method?: DigitalDeliveryMethod;
  shipping_regions?: string[];
};

export type RefundRules = {
  refund_window_days: number;
  is_refundable: boolean;
  summary: string;
};

export type ProductDetail = {
  product_id: string;
  seller_id: string;
  store_id?: string;
  store_name: string;
  title: string;
  description: string;
  fulfillment_type: FulfillmentType;
  price_minor: number;
  currency: CurrencyCode;
  available_stock: number;
  status: ProductStatus;
  delivery_rules: DeliveryRules;
  refund_rules: RefundRules;
  variants: ProductVariant[];
};

export type CartItem = {
  cart_item_id: string;
  product_id: string;
  variant_id: string;
  title: string;
  unit_price_minor: number;
  currency: CurrencyCode;
  quantity: number;
  source: CartItemSource;
  fulfillment_type: FulfillmentType;
  available_stock: number;
};

export type Cart = {
  cart_id: string;
  items: CartItem[];
};

export type PhysicalShippingInfo = {
  destination_region: string;
  method: ShippingMethod | string;
  estimated_days_min: number;
  estimated_days_max: number;
};

export type CheckoutLineItem = {
  product_id: string;
  variant_id: string;
  title: string;
  quantity: number;
  snapshot_unit_price_minor: number;
  current_unit_price_minor: number;
  currency: CurrencyCode;
  fulfillment_type: FulfillmentType;
  available_stock: number;
  source: CartItemSource;
  digital_delivery_method?: DigitalDeliveryMethod;
  shipping?: PhysicalShippingInfo;
};

export type CheckoutPreview = {
  items: CheckoutLineItem[];
  subtotal_minor: number;
  shipping_fee_minor: number;
  tax_minor: number;
  total_minor: number;
  currency: CurrencyCode;
  expires_at: string;
  requires_price_reconfirm: boolean;
  inventory_is_available: boolean;
  payment_provider_status: PaymentProviderRedirectStatus;
};

export type OrderLineItem = {
  order_item_id: string;
  product_id: string;
  variant_id: string;
  title: string;
  quantity: number;
  unit_price_minor: number;
  currency: CurrencyCode;
  fulfillment_type: FulfillmentType;
};

export type OrderDetail = {
  order_id: string;
  order_status: OrderStatus;
  payment_status: PaymentStatus;
  fulfillment_status: FulfillmentStatus;
  shipment_status: ShipmentStatus | null;
  items: OrderLineItem[];
  subtotal_minor: number;
  shipping_fee_minor: number;
  tax_minor: number;
  total_minor: number;
  currency: CurrencyCode;
  created_at: string;
  payment_redirect_url?: string | null;
  payment_intent_url?: string | null;
  redirect_url?: string | null;
};

export type FileSecurityCheck = {
  status: FileSecurityCheckStatus;
  summary: string;
};

export type SellerProductDraft = {
  title: string;
  description: string;
  fulfillment_type: FulfillmentType;
  price_minor: number;
  currency: CurrencyCode;
  available_stock: number;
  refund_window_days: number;
  digital?: {
    delivery_method: DigitalDeliveryMethod;
    file_name?: string;
    file_security_check: FileSecurityCheck;
  };
  physical?: {
    weight_grams: number;
    length_mm: number;
    width_mm: number;
    height_mm: number;
    shipping_regions: string[];
    shipping_method: ShippingMethod;
    estimated_days_min: number;
    estimated_days_max: number;
  };
};
