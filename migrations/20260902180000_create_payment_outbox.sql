CREATE TABLE payments (
    payment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL UNIQUE REFERENCES orders (order_id),
    provider_name TEXT NOT NULL,
    provider_payment_id TEXT,
    amount_minor BIGINT NOT NULL,
    currency CHAR(3) NOT NULL,
    status TEXT NOT NULL DEFAULT 'created',
    refunded_amount_minor BIGINT NOT NULL DEFAULT 0,
    last_fact_occurred_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT payments_amount_non_negative CHECK (amount_minor >= 0),
    CONSTRAINT payments_refunded_amount_non_negative CHECK (refunded_amount_minor >= 0),
    CONSTRAINT payments_refunded_amount_not_above_total CHECK (refunded_amount_minor <= amount_minor),
    CONSTRAINT payments_currency_check CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT payments_status_check CHECK (
        status IN ('created', 'processing', 'succeeded', 'failed', 'refund_processing', 'refunded')
    )
);

CREATE TABLE payment_events (
    provider_event_id TEXT PRIMARY KEY,
    payment_id UUID NOT NULL REFERENCES payments (payment_id),
    order_id UUID NOT NULL REFERENCES orders (order_id),
    event_kind TEXT NOT NULL,
    payload JSONB NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT payment_events_kind_check CHECK (
        event_kind IN ('payment_succeeded', 'refund_succeeded')
    )
);

CREATE TABLE refunds (
    refund_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payment_id UUID NOT NULL REFERENCES payments (payment_id),
    order_id UUID NOT NULL REFERENCES orders (order_id),
    amount_minor BIGINT NOT NULL,
    currency CHAR(3) NOT NULL,
    status TEXT NOT NULL DEFAULT 'requested',
    provider_refund_id TEXT,
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT refunds_amount_positive CHECK (amount_minor > 0),
    CONSTRAINT refunds_currency_check CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT refunds_status_check CHECK (
        status IN ('requested', 'processing', 'succeeded', 'failed')
    )
);

CREATE TABLE outbox_events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    available_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT outbox_events_status_check CHECK (status IN ('pending', 'published', 'dead_letter')),
    CONSTRAINT outbox_events_attempts_non_negative CHECK (attempts >= 0)
);

CREATE INDEX payment_events_payment_idx ON payment_events (payment_id, received_at DESC);
CREATE INDEX refunds_order_idx ON refunds (order_id, created_at DESC);
CREATE INDEX outbox_pending_idx ON outbox_events (status, available_at, created_at);
