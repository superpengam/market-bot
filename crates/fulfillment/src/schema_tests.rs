const FULFILLMENT_SETTLEMENT_MIGRATION: &str =
    include_str!("../../../migrations/20260902200000_create_fulfillment_settlement.sql");

#[test]
fn should_persist_encrypted_digital_assets_and_unique_fulfillments() {
    assert!(
        FULFILLMENT_SETTLEMENT_MIGRATION.contains("encrypted_reference"),
        "digital assets must persist only encrypted references"
    );
    assert!(
        FULFILLMENT_SETTLEMENT_MIGRATION.contains("assigned_order_id"),
        "one-time credentials must record the assigned order"
    );
    assert!(
        FULFILLMENT_SETTLEMENT_MIGRATION.contains("UNIQUE"),
        "an order may have only one fulfillment and one settlement"
    );
}

#[test]
fn should_constrain_shipment_and_settlement_status_values() {
    assert!(
        FULFILLMENT_SETTLEMENT_MIGRATION.contains("label_created"),
        "shipments must constrain the platform logistics status set"
    );
    assert!(
        FULFILLMENT_SETTLEMENT_MIGRATION.contains("digital_delivered_at"),
        "settlements must record digital delivery before eligibility"
    );
    assert!(
        FULFILLMENT_SETTLEMENT_MIGRATION.contains("blocked_reason"),
        "settlements must persist why release is blocked"
    );
}
