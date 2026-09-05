-- Product reviews, file scans, and user reports. Audit columns store actor,
-- reason, and time only -- never raw card secrets, payment tokens, or addresses.
CREATE TABLE product_reviews (
    product_review_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products (product_id),
    decision TEXT NOT NULL,
    reason TEXT NOT NULL,
    actor_user_id UUID NOT NULL REFERENCES users (user_id),
    decided_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT product_reviews_decision_check CHECK (
        decision IN ('approved', 'rejected', 'suspended', 'needs_review')
    ),
    CONSTRAINT product_reviews_reason_not_blank CHECK (length(btrim(reason)) > 0)
);

CREATE INDEX product_reviews_product_decided_idx
    ON product_reviews (product_id, decided_at DESC);

CREATE TABLE file_scan_results (
    file_scan_result_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products (product_id),
    asset_id UUID NOT NULL,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    verdict TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    scanned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT file_scan_results_verdict_check CHECK (
        verdict IN ('passed', 'failed', 'needs_review')
    ),
    CONSTRAINT file_scan_results_filename_not_blank CHECK (length(btrim(filename)) > 0),
    CONSTRAINT file_scan_results_reason_not_blank CHECK (length(btrim(reason_code)) > 0)
);

CREATE INDEX file_scan_results_product_scanned_idx
    ON file_scan_results (product_id, scanned_at DESC);

CREATE TABLE reports (
    report_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_user_id UUID NOT NULL REFERENCES users (user_id),
    subject_type TEXT NOT NULL,
    subject_id UUID NOT NULL,
    reason_code TEXT NOT NULL,
    details TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT reports_subject_type_check CHECK (
        subject_type IN ('product', 'seller', 'user')
    ),
    CONSTRAINT reports_status_check CHECK (
        status IN ('open', 'in_review', 'resolved', 'rejected')
    ),
    CONSTRAINT reports_reason_code_not_blank CHECK (length(btrim(reason_code)) > 0),
    CONSTRAINT reports_details_not_blank CHECK (length(btrim(details)) > 0)
);

CREATE INDEX reports_status_created_idx ON reports (status, created_at DESC);
CREATE INDEX reports_subject_idx ON reports (subject_type, subject_id);

CREATE TABLE report_actions (
    report_action_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id UUID NOT NULL REFERENCES reports (report_id) ON DELETE CASCADE,
    actor_user_id UUID NOT NULL REFERENCES users (user_id),
    decision TEXT NOT NULL,
    reason TEXT NOT NULL,
    acted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT report_actions_decision_check CHECK (
        decision IN ('approved', 'rejected', 'suspended', 'needs_review')
    ),
    CONSTRAINT report_actions_reason_not_blank CHECK (length(btrim(reason)) > 0)
);

CREATE INDEX report_actions_report_acted_idx
    ON report_actions (report_id, acted_at DESC);

CREATE TABLE moderation_cases (
    moderation_case_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_kind TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id UUID NOT NULL,
    report_id UUID REFERENCES reports (report_id),
    status TEXT NOT NULL DEFAULT 'open',
    decision TEXT,
    reason TEXT,
    actor_user_id UUID REFERENCES users (user_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ,
    CONSTRAINT moderation_cases_kind_check CHECK (
        case_kind IN ('product_review', 'user_report', 'file_scan')
    ),
    CONSTRAINT moderation_cases_subject_type_check CHECK (
        subject_type IN ('product', 'seller', 'user')
    ),
    CONSTRAINT moderation_cases_status_check CHECK (
        status IN ('open', 'in_review', 'resolved')
    ),
    CONSTRAINT moderation_cases_decision_check CHECK (
        decision IS NULL
        OR decision IN ('approved', 'rejected', 'suspended', 'needs_review')
    )
);

CREATE INDEX moderation_cases_subject_idx
    ON moderation_cases (subject_type, subject_id, created_at DESC);

CREATE TABLE moderation_actions (
    moderation_action_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    moderation_case_id UUID NOT NULL REFERENCES moderation_cases (moderation_case_id)
        ON DELETE CASCADE,
    actor_user_id UUID NOT NULL REFERENCES users (user_id),
    decision TEXT NOT NULL,
    reason TEXT NOT NULL,
    acted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT moderation_actions_decision_check CHECK (
        decision IN ('approved', 'rejected', 'suspended', 'needs_review')
    ),
    CONSTRAINT moderation_actions_reason_not_blank CHECK (length(btrim(reason)) > 0)
);

CREATE INDEX moderation_actions_case_acted_idx
    ON moderation_actions (moderation_case_id, acted_at DESC);
