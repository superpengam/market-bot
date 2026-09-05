-- Digital assets store only encrypted references. Card secrets and redeem
-- codes are assigned to at most one order. Settlement rows stay in payment
-- status independently of order and shipment state.
CREATE TABLE digital_assets (
    digital_asset_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products (product_id),
    asset_type TEXT NOT NULL,
    encrypted_reference TEXT NOT NULL,
    assigned_order_id UUID REFERENCES orders (order_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT digital_assets_type_check CHECK (
        asset_type IN ('file', 'card_secret', 'redeem_code')
    ),
    CONSTRAINT digital_assets_encrypted_reference_not_blank CHECK (
        length(btrim(encrypted_reference)) > 0
    )
);

CREATE INDEX digital_assets_product_unassigned_idx
    ON digital_assets (product_id, assigned_order_id);

-- Invariant: one paid order can own at most one assigned one-time credential.
CREATE UNIQUE INDEX digital_assets_assigned_order_unique
    ON digital_assets (assigned_order_id)
    WHERE assigned_order_id IS NOT NULL;

CREATE TABLE fulfillments (
    fulfillment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL UNIQUE REFERENCES orders (order_id),
    status TEXT NOT NULL DEFAULT 'pending',
    download_expires_at TIMESTAMPTZ,
    download_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT fulfillments_status_check CHECK (
        status IN ('pending', 'delivered', 'failed')
    ),
    CONSTRAINT fulfillments_download_count_non_negative CHECK (download_count >= 0)
);

CREATE TABLE delivery_attempts (
    delivery_attempt_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fulfillment_id UUID NOT NULL REFERENCES fulfillments (fulfillment_id),
    order_id UUID NOT NULL REFERENCES orders (order_id),
    outcome TEXT NOT NULL,
    error_code TEXT,
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT delivery_attempts_outcome_check CHECK (
        outcome IN ('succeeded', 'failed', 'retrying')
    )
);

CREATE INDEX delivery_attempts_fulfillment_idx
    ON delivery_attempts (fulfillment_id, attempted_at DESC);

CREATE TABLE shipments (
    shipment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES orders (order_id),
    tracking_number TEXT NOT NULL,
    carrier TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'label_created',
    last_synced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT shipments_status_check CHECK (
        status IN ('label_created', 'in_transit', 'delivered', 'exception', 'returned')
    ),
    CONSTRAINT shipments_tracking_not_blank CHECK (length(btrim(tracking_number)) > 0),
    CONSTRAINT shipments_carrier_not_blank CHECK (length(btrim(carrier)) > 0)
);

CREATE INDEX shipments_order_idx ON shipments (order_id, created_at DESC);
CREATE INDEX shipments_tracking_idx ON shipments (tracking_number);

CREATE TABLE settlements (
    settlement_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL UNIQUE REFERENCES orders (order_id),
    seller_id UUID NOT NULL REFERENCES seller_profiles (seller_id),
    amount_minor BIGINT NOT NULL,
    currency CHAR(3) NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    eligible_at TIMESTAMPTZ,
    provider_settlement_id TEXT,
    blocked_reason TEXT,
    digital_delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT settlements_amount_non_negative CHECK (amount_minor >= 0),
    CONSTRAINT settlements_currency_check CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT settlements_status_check CHECK (
        status IN ('pending', 'eligible', 'released', 'blocked', 'failed')
    ),
    CONSTRAINT settlements_blocked_reason_check CHECK (
        blocked_reason IS NULL OR blocked_reason IN ('refund', 'dispute', 'logistics_exception')
    )
);

CREATE INDEX settlements_seller_status_idx ON settlements (seller_id, status, created_at DESC);
