use market_bot_shared::{FulfillmentType, Money, ProductId, SellerId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Product {
    id: ProductId,
    seller_id: SellerId,
    title: String,
    description: String,
    product_type: ProductType,
    price: Money,
    status: ProductStatus,
}

impl Product {
    pub fn new(
        seller_id: SellerId,
        title: String,
        description: String,
        product_type: ProductType,
        price: Money,
    ) -> Result<Self, ProductError> {
        let title = title.trim().to_owned();
        if title.is_empty() {
            return Err(ProductError::BlankTitle);
        }

        if description.trim().is_empty() {
            return Err(ProductError::BlankDescription);
        }

        Ok(Self {
            id: ProductId::new(),
            seller_id,
            title,
            description: description.trim().to_owned(),
            product_type,
            price,
            status: ProductStatus::Draft,
        })
    }

    pub const fn id(&self) -> ProductId {
        self.id
    }

    pub const fn seller_id(&self) -> SellerId {
        self.seller_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub const fn product_type(&self) -> ProductType {
        self.product_type
    }

    pub const fn price(&self) -> &Money {
        &self.price
    }

    pub const fn status(&self) -> ProductStatus {
        self.status
    }

    pub const fn is_publicly_visible(&self) -> bool {
        matches!(self.status, ProductStatus::Published)
    }

    pub const fn can_be_added_to_cart(&self) -> bool {
        matches!(self.status, ProductStatus::Published)
    }

    pub fn submit_for_review(&mut self) -> Result<(), ProductError> {
        if self.status != ProductStatus::Draft {
            return Err(ProductError::InvalidStatusTransition);
        }

        self.status = ProductStatus::PendingReview;
        Ok(())
    }

    pub fn publish(&mut self) -> Result<(), ProductError> {
        if self.status != ProductStatus::PendingReview {
            return Err(ProductError::InvalidStatusTransition);
        }

        self.status = ProductStatus::Published;
        Ok(())
    }

    pub fn suspend(&mut self) -> Result<(), ProductError> {
        if !matches!(
            self.status,
            ProductStatus::Published | ProductStatus::PendingReview
        ) {
            return Err(ProductError::InvalidStatusTransition);
        }

        self.status = ProductStatus::Suspended;
        Ok(())
    }

    pub fn return_to_review(&mut self) -> Result<(), ProductError> {
        if self.status != ProductStatus::Suspended {
            return Err(ProductError::InvalidStatusTransition);
        }

        self.status = ProductStatus::PendingReview;
        Ok(())
    }
}

pub type ProductType = FulfillmentType;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProductStatus {
    Draft,
    PendingReview,
    Published,
    Suspended,
    Archived,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ProductError {
    #[error("product title cannot be blank")]
    BlankTitle,
    #[error("product description cannot be blank")]
    BlankDescription,
    #[error("product status transition is invalid")]
    InvalidStatusTransition,
}
