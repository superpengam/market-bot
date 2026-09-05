CREATE TABLE carts (
    cart_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id UUID NOT NULL REFERENCES users (user_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE cart_items (
    cart_item_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cart_id UUID NOT NULL REFERENCES carts (cart_id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products (product_id),
    product_variant_id UUID NOT NULL REFERENCES product_variants (product_variant_id),
    title_snapshot TEXT NOT NULL,
    unit_price_minor BIGINT NOT NULL,
    currency CHAR(3) NOT NULL,
    quantity BIGINT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    fulfillment_type TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT cart_items_quantity_positive CHECK (quantity > 0),
    CONSTRAINT cart_items_source_check CHECK (source IN ('user', 'ai')),
    CONSTRAINT cart_items_fulfillment_type_check CHECK (
        fulfillment_type IN ('digital', 'physical_standard')
    ),
    CONSTRAINT cart_items_price_non_negative CHECK (unit_price_minor >= 0),
    CONSTRAINT cart_items_currency_check CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT cart_items_cart_variant_unique UNIQUE (cart_id, product_variant_id)
);

CREATE INDEX carts_owner_idx ON carts (owner_user_id);
CREATE INDEX cart_items_cart_idx ON cart_items (cart_id);

CREATE TABLE orders (
    order_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    buyer_user_id UUID NOT NULL REFERENCES users (user_id),
    status TEXT NOT NULL DEFAULT 'draft',
    subtotal_minor BIGINT NOT NULL,
    shipping_fee_minor BIGINT NOT NULL,
    tax_minor BIGINT NOT NULL,
    total_minor BIGINT NOT NULL,
    currency CHAR(3) NOT NULL,
    idempotency_key TEXT NOT NULL,
    request_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT orders_status_check CHECK (
        status IN (
            'draft', 'pending_confirmation', 'pending_payment', 'payment_processing',
            'paid', 'fulfillment_processing', 'shipped', 'delivered', 'completed',
            'cancellation_requested', 'cancelled', 'refund_processing', 'refunded',
            'dispute_processing'
        )
    ),
    CONSTRAINT orders_amounts_non_negative CHECK (
        subtotal_minor >= 0 AND shipping_fee_minor >= 0 AND tax_minor >= 0 AND total_minor >= 0
    ),
    CONSTRAINT orders_currency_check CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT orders_idempotency_key_not_blank CHECK (length(btrim(idempotency_key)) > 0),
    CONSTRAINT orders_buyer_idempotency_unique UNIQUE (buyer_user_id, idempotency_key)
);

CREATE TABLE order_items (
    order_item_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES orders (order_id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products (product_id),
    product_variant_id UUID NOT NULL REFERENCES product_variants (product_variant_id),
    seller_id UUID NOT NULL REFERENCES seller_profiles (seller_id),
    title_snapshot TEXT NOT NULL,
    unit_price_minor BIGINT NOT NULL,
    currency CHAR(3) NOT NULL,
    quantity BIGINT NOT NULL,
    fulfillment_type TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT order_items_quantity_positive CHECK (quantity > 0),
    CONSTRAINT order_items_price_non_negative CHECK (unit_price_minor >= 0),
    CONSTRAINT order_items_currency_check CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT order_items_fulfillment_type_check CHECK (
        fulfillment_type IN ('digital', 'physical_standard')
    )
);

CREATE TABLE order_state_transitions (
    transition_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES orders (order_id) ON DELETE CASCADE,
    from_status TEXT NOT NULL,
    to_status TEXT NOT NULL,
    actor_type TEXT NOT NULL,
    actor_id UUID,
    request_id UUID NOT NULL,
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX orders_buyer_created_idx ON orders (buyer_user_id, created_at DESC);
CREATE INDEX order_items_order_idx ON order_items (order_id);
CREATE INDEX order_transitions_order_created_idx
    ON order_state_transitions (order_id, created_at DESC);
