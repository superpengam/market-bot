//! Product search domain module.

mod in_memory_repository;
mod ports;
mod search_products;

pub use in_memory_repository::InMemorySearchRepository;
pub use ports::{SearchRepository, SearchRepositoryError};
pub use search_products::{
    DEFAULT_PAGE_SIZE, ProductSearchDocument, ProductSearchResult, SearchError,
    SearchProductsQuery, SearchService,
};

#[cfg(test)]
mod search_tests;
