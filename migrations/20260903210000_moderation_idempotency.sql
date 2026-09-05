-- Replay store for admin review, suspend, report create, and report resolve.
CREATE TABLE moderation_idempotency_keys (
    actor_user_id UUID NOT NULL REFERENCES users (user_id),
    scope TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    response_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (actor_user_id, scope, idempotency_key),
    CONSTRAINT moderation_idempotency_scope_not_blank CHECK (length(btrim(scope)) > 0),
    CONSTRAINT moderation_idempotency_key_not_blank CHECK (length(btrim(idempotency_key)) > 0)
);
