//! Shopping cart domain module.

mod cart;
mod cart_service;
mod in_memory_repository;
mod ports;

pub use cart::{AddCartItem, Cart, CartError, CartItem, CartItemSource};
pub use cart_service::{CartService, CartServiceError};
pub use in_memory_repository::InMemoryCartRepository;
pub use ports::{CartRepository, CartRepositoryError};

#[cfg(test)]
mod application_tests;
#[cfg(test)]
mod cart_tests;
