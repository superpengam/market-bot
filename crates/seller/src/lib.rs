//! Seller and store domain module.

mod seller_profile;
mod store;

pub use seller_profile::{SellerProfile, SellerStatus};
pub use store::{Store, StoreError};

#[cfg(test)]
mod store_tests;
