use market_bot_shared::{ProductVariantId, StockReservationId};

use super::{Inventory, InventoryError};

#[test]
fn should_reserve_available_stock() {
    let variant_id = ProductVariantId::new();
    let mut inventory = Inventory::new(variant_id, 3);
    let reservation_id = StockReservationId::new();

    inventory
        .reserve(2, reservation_id)
        .expect("available stock should be reservable");

    assert_eq!(inventory.available_stock(), 1);
    assert_eq!(inventory.reserved_stock(), 2);
}

#[test]
fn should_reject_reservation_when_stock_is_insufficient() {
    let variant_id = ProductVariantId::new();
    let mut inventory = Inventory::new(variant_id, 1);

    let result = inventory.reserve(2, StockReservationId::new());

    assert_eq!(result, Err(InventoryError::InsufficientStock));
    assert_eq!(inventory.available_stock(), 1);
    assert_eq!(inventory.reserved_stock(), 0);
}

#[test]
fn should_make_repeated_reservation_idempotent() {
    let variant_id = ProductVariantId::new();
    let mut inventory = Inventory::new(variant_id, 3);
    let reservation_id = StockReservationId::new();

    inventory
        .reserve(2, reservation_id)
        .expect("first reservation should work");
    inventory
        .reserve(2, reservation_id)
        .expect("retrying the same reservation should work");

    assert_eq!(inventory.available_stock(), 1);
    assert_eq!(inventory.reserved_stock(), 2);
}

#[test]
fn should_release_a_reservation_once() {
    let variant_id = ProductVariantId::new();
    let mut inventory = Inventory::new(variant_id, 3);
    let reservation_id = StockReservationId::new();

    inventory
        .reserve(2, reservation_id)
        .expect("reservation should work");
    inventory
        .release(reservation_id)
        .expect("release should work");
    inventory
        .release(reservation_id)
        .expect("repeated release should be idempotent");

    assert_eq!(inventory.available_stock(), 3);
    assert_eq!(inventory.reserved_stock(), 0);
}
