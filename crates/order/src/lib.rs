//! Order lifecycle domain module.

mod in_memory_repository;
mod order;
mod order_service;
mod ports;

pub use in_memory_repository::InMemoryOrderRepository;
pub use order::{Order, OrderError, OrderItem, OrderStatus};
pub use order_service::{CreateOrderCommand, OrderService, OrderServiceError};
pub use ports::{OrderRepository, OrderRepositoryError};

#[cfg(test)]
mod application_tests;
#[cfg(test)]
mod order_tests;
