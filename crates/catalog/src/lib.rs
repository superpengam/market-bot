//! Product catalog and inventory domain module.

mod catalog_service;
mod in_memory_repository;
mod inventory;
mod ports;
mod product;
mod product_variant;

pub use catalog_service::{CatalogError, CatalogService, CreateProductCommand};
pub use in_memory_repository::InMemoryCatalogRepository;
pub use inventory::{Inventory, InventoryError, StockReservation};
pub use ports::{CatalogRepository, CatalogRepositoryError};
pub use product::{Product, ProductError, ProductStatus, ProductType};
pub use product_variant::{ProductVariant, ProductVariantError};

#[cfg(test)]
mod application_tests;
#[cfg(test)]
mod inventory_tests;
#[cfg(test)]
mod product_tests;
