use market_bot_shared::{
    CartId, CartItemId, FulfillmentType, Money, ProductId, ProductVariantId, UserId,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddCartItem {
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
    pub title: String,
    pub unit_price: Money,
    pub quantity: u64,
    pub source: CartItemSource,
    pub fulfillment_type: FulfillmentType,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Cart {
    id: CartId,
    owner_id: UserId,
    items: Vec<CartItem>,
}

impl Cart {
    pub fn new(owner_id: UserId) -> Self {
        Self {
            id: CartId::new(),
            owner_id,
            items: Vec::new(),
        }
    }

    pub const fn id(&self) -> CartId {
        self.id
    }

    pub const fn owner_id(&self) -> UserId {
        self.owner_id
    }

    pub fn items(&self) -> &[CartItem] {
        &self.items
    }

    pub fn add_item(&mut self, item: AddCartItem) -> Result<CartItem, CartError> {
        if item.quantity == 0 {
            return Err(CartError::InvalidQuantity);
        }

        if let Some(existing) = self
            .items
            .iter_mut()
            .find(|existing| existing.variant_id == item.variant_id)
        {
            if existing.unit_price != item.unit_price {
                return Err(CartError::PriceSnapshotChanged);
            }

            existing.quantity = existing
                .quantity
                .checked_add(item.quantity)
                .ok_or(CartError::QuantityOverflow)?;
            return Ok(existing.clone());
        }

        let cart_item = CartItem {
            id: CartItemId::new(),
            product_id: item.product_id,
            variant_id: item.variant_id,
            title: item.title.trim().to_owned(),
            unit_price: item.unit_price,
            quantity: item.quantity,
            source: item.source,
            fulfillment_type: item.fulfillment_type,
        };
        if cart_item.title.is_empty() {
            return Err(CartError::BlankTitle);
        }

        self.items.push(cart_item.clone());
        Ok(cart_item)
    }

    pub fn update_quantity(&mut self, item_id: CartItemId, quantity: u64) -> Result<(), CartError> {
        if quantity == 0 {
            return Err(CartError::InvalidQuantity);
        }

        let item = self
            .items
            .iter_mut()
            .find(|item| item.id == item_id)
            .ok_or(CartError::ItemNotFound)?;
        item.quantity = quantity;
        Ok(())
    }

    pub fn remove_item(&mut self, item_id: CartItemId) -> bool {
        let original_length = self.items.len();
        self.items.retain(|item| item.id != item_id);
        self.items.len() != original_length
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CartItem {
    id: CartItemId,
    product_id: ProductId,
    variant_id: ProductVariantId,
    title: String,
    unit_price: Money,
    quantity: u64,
    source: CartItemSource,
    fulfillment_type: FulfillmentType,
}

impl CartItem {
    pub const fn id(&self) -> CartItemId {
        self.id
    }

    pub const fn product_id(&self) -> ProductId {
        self.product_id
    }

    pub const fn variant_id(&self) -> ProductVariantId {
        self.variant_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub const fn unit_price(&self) -> &Money {
        &self.unit_price
    }

    pub const fn quantity(&self) -> u64 {
        self.quantity
    }

    pub const fn source(&self) -> CartItemSource {
        self.source
    }

    pub const fn fulfillment_type(&self) -> FulfillmentType {
        self.fulfillment_type
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CartItemSource {
    User,
    Ai,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum CartError {
    #[error("cart item quantity must be greater than zero")]
    InvalidQuantity,
    #[error("cart item title cannot be blank")]
    BlankTitle,
    #[error("cart item price snapshot changed")]
    PriceSnapshotChanged,
    #[error("cart item quantity overflowed")]
    QuantityOverflow,
    #[error("cart item was not found")]
    ItemNotFound,
}
