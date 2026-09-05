use market_bot_shared::{Money, ProductId, ProductVariantId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProductVariant {
    id: ProductVariantId,
    product_id: ProductId,
    sku: String,
    price: Money,
}

impl ProductVariant {
    pub fn new(
        product_id: ProductId,
        sku: String,
        price: Money,
    ) -> Result<Self, ProductVariantError> {
        let sku = sku.trim().to_owned();
        if sku.is_empty() {
            return Err(ProductVariantError::BlankSku);
        }

        Ok(Self {
            id: ProductVariantId::new(),
            product_id,
            sku,
            price,
        })
    }

    pub const fn id(&self) -> ProductVariantId {
        self.id
    }

    pub const fn product_id(&self) -> ProductId {
        self.product_id
    }

    pub fn sku(&self) -> &str {
        &self.sku
    }

    pub const fn price(&self) -> &Money {
        &self.price
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ProductVariantError {
    #[error("product SKU cannot be blank")]
    BlankSku,
}
