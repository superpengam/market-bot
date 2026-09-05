use market_bot_shared::{CurrencyCode, Money, ProductId, ProductVariantId, StockReservationId};

use crate::{
    CatalogRepository, CatalogRepositoryError, Inventory, Product, ProductError, ProductType,
};

#[derive(Clone, Debug)]
pub struct CreateProductCommand {
    pub seller_id: market_bot_shared::SellerId,
    pub title: String,
    pub description: String,
    pub product_type: ProductType,
    pub price_minor: i64,
    pub currency: String,
}

#[derive(Clone)]
pub struct CatalogService<R> {
    repository: R,
}

impl<R> CatalogService<R>
where
    R: CatalogRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn create_product(
        &self,
        command: CreateProductCommand,
    ) -> Result<Product, CatalogError> {
        let currency =
            CurrencyCode::try_from(command.currency).map_err(CatalogError::InvalidCurrencyCode)?;
        let price =
            Money::new(command.price_minor, currency).map_err(CatalogError::InvalidPrice)?;
        let product = Product::new(
            command.seller_id,
            command.title,
            command.description,
            command.product_type,
            price,
        )
        .map_err(CatalogError::InvalidProduct)?;

        self.repository
            .save_product(product.clone())
            .await
            .map_err(CatalogError::Repository)?;
        Ok(product)
    }

    pub async fn get_product(
        &self,
        product_id: ProductId,
    ) -> Result<Option<Product>, CatalogError> {
        self.repository
            .find_product(product_id)
            .await
            .map_err(CatalogError::Repository)
    }

    pub async fn initialize_inventory(
        &self,
        variant_id: ProductVariantId,
        available_stock: u64,
    ) -> Result<(), CatalogError> {
        self.repository
            .save_inventory(Inventory::new(variant_id, available_stock))
            .await
            .map_err(CatalogError::Repository)
    }

    pub async fn get_inventory(
        &self,
        variant_id: ProductVariantId,
    ) -> Result<Option<Inventory>, CatalogError> {
        self.repository
            .find_inventory(variant_id)
            .await
            .map_err(CatalogError::Repository)
    }

    pub async fn reserve_stock(
        &self,
        variant_id: ProductVariantId,
        quantity: u64,
        reservation_id: StockReservationId,
    ) -> Result<(), CatalogError> {
        self.repository
            .reserve_stock(variant_id, quantity, reservation_id)
            .await
            .map_err(CatalogError::Repository)
    }

    pub async fn release_stock(
        &self,
        variant_id: ProductVariantId,
        reservation_id: StockReservationId,
    ) -> Result<(), CatalogError> {
        self.repository
            .release_stock(variant_id, reservation_id)
            .await
            .map_err(CatalogError::Repository)
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum CatalogError {
    #[error("product is invalid: {0}")]
    InvalidProduct(#[source] ProductError),
    #[error("price is invalid: {0}")]
    InvalidPrice(#[source] market_bot_shared::MoneyError),
    #[error("currency code is invalid: {0}")]
    InvalidCurrencyCode(#[source] market_bot_shared::CurrencyCodeError),
    #[error("catalog repository failed: {0}")]
    Repository(#[source] CatalogRepositoryError),
}
