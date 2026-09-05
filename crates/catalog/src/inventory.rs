use std::collections::HashMap;

use market_bot_shared::{ProductVariantId, StockReservationId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Inventory {
    variant_id: ProductVariantId,
    available_stock: u64,
    reservations: HashMap<StockReservationId, u64>,
}

impl Inventory {
    pub fn new(variant_id: ProductVariantId, available_stock: u64) -> Self {
        Self {
            variant_id,
            available_stock,
            reservations: HashMap::new(),
        }
    }

    pub const fn variant_id(&self) -> ProductVariantId {
        self.variant_id
    }

    pub const fn available_stock(&self) -> u64 {
        self.available_stock
    }

    pub fn reserved_stock(&self) -> u64 {
        self.reservations.values().sum()
    }

    pub fn reserve(
        &mut self,
        quantity: u64,
        reservation_id: StockReservationId,
    ) -> Result<StockReservation, InventoryError> {
        if quantity == 0 {
            return Err(InventoryError::InvalidQuantity);
        }

        if let Some(existing_quantity) = self.reservations.get(&reservation_id) {
            if *existing_quantity == quantity {
                return Ok(StockReservation {
                    reservation_id,
                    variant_id: self.variant_id,
                    quantity,
                });
            }

            return Err(InventoryError::ReservationConflict);
        }

        if self.available_stock < quantity {
            return Err(InventoryError::InsufficientStock);
        }

        self.available_stock -= quantity;
        self.reservations.insert(reservation_id, quantity);

        Ok(StockReservation {
            reservation_id,
            variant_id: self.variant_id,
            quantity,
        })
    }

    pub fn release(&mut self, reservation_id: StockReservationId) -> Result<(), InventoryError> {
        if let Some(quantity) = self.reservations.remove(&reservation_id) {
            self.available_stock = self
                .available_stock
                .checked_add(quantity)
                .ok_or(InventoryError::StockOverflow)?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StockReservation {
    pub reservation_id: StockReservationId,
    pub variant_id: ProductVariantId,
    pub quantity: u64,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum InventoryError {
    #[error("inventory reservation quantity must be greater than zero")]
    InvalidQuantity,
    #[error("inventory does not have enough available stock")]
    InsufficientStock,
    #[error("reservation ID was already used with a different quantity")]
    ReservationConflict,
    #[error("inventory stock value overflowed")]
    StockOverflow,
}
