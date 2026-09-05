const PAYMENT_OUTBOX_MIGRATION: &str =
    include_str!("../../../migrations/20260902180000_create_payment_outbox.sql");

#[test]
fn should_persist_refunded_amount_and_last_fact_time_on_payments() {
    assert!(
        PAYMENT_OUTBOX_MIGRATION.contains("refunded_amount_minor"),
        "payments must persist refunded_amount_minor so partial refunds survive a SQL adapter"
    );
    assert!(
        PAYMENT_OUTBOX_MIGRATION.contains("last_fact_occurred_at"),
        "payments must persist last_fact_occurred_at so stale webhook facts can be rejected"
    );
}
