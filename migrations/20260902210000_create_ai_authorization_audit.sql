-- AI grants, purchase policies, and action audits. Audit text must never store
-- card numbers, payment tokens, or unnecessary personal addresses.
CREATE TABLE ai_authorizations (
    ai_authorization_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_user_id UUID NOT NULL REFERENCES users (user_id),
    client_id TEXT NOT NULL,
    scopes TEXT[] NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ai_authorizations_client_id_not_blank CHECK (length(btrim(client_id)) > 0),
    CONSTRAINT ai_authorizations_scopes_known CHECK (
        scopes <@ ARRAY[
            'catalog:read',
            'cart:read',
            'cart:write',
            'checkout:preview',
            'order:create',
            'order:read',
            'order:auto_purchase'
        ]::TEXT[]
    )
);

CREATE INDEX ai_authorizations_subject_expires_idx
    ON ai_authorizations (subject_user_id, expires_at DESC);

CREATE TABLE ai_policies (
    ai_policy_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_user_id UUID NOT NULL UNIQUE REFERENCES users (user_id),
    allowed_categories TEXT[] NOT NULL,
    max_order_minor BIGINT NOT NULL,
    max_daily_minor BIGINT NOT NULL,
    max_monthly_minor BIGINT NOT NULL,
    max_shipping_minor BIGINT NOT NULL,
    currency CHAR(3) NOT NULL,
    allowed_seller_score INTEGER NOT NULL,
    require_price_reconfirmation BOOLEAN NOT NULL DEFAULT TRUE,
    is_auto_purchase_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ai_policies_amounts_non_negative CHECK (
        max_order_minor >= 0
        AND max_daily_minor >= 0
        AND max_monthly_minor >= 0
        AND max_shipping_minor >= 0
    ),
    CONSTRAINT ai_policies_currency_check CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT ai_policies_seller_score_check CHECK (allowed_seller_score >= 0)
);

CREATE TABLE ai_actions (
    ai_action_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ai_authorization_id UUID REFERENCES ai_authorizations (ai_authorization_id),
    subject_user_id UUID REFERENCES users (user_id),
    action_type TEXT NOT NULL,
    input_summary TEXT NOT NULL,
    result TEXT NOT NULL,
    request_id UUID NOT NULL,
    order_id UUID REFERENCES orders (order_id),
    error_code TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ai_actions_type_check CHECK (
        action_type IN (
            'authorize',
            'revoke',
            'search_products',
            'add_to_cart',
            'auto_purchase'
        )
    ),
    CONSTRAINT ai_actions_result_check CHECK (
        result IN (
            'succeeded',
            'requires_user_confirmation',
            'blocked',
            'failed'
        )
    ),
    CONSTRAINT ai_actions_input_summary_not_blank CHECK (length(btrim(input_summary)) > 0)
);

CREATE INDEX ai_actions_subject_created_idx
    ON ai_actions (subject_user_id, created_at DESC);
CREATE INDEX ai_actions_authorization_created_idx
    ON ai_actions (ai_authorization_id, created_at DESC);
CREATE INDEX ai_actions_request_id_idx ON ai_actions (request_id);
