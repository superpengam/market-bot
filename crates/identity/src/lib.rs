//! User identity domain module.

mod user;

pub use user::{User, UserError, UserStatus};

#[cfg(test)]
mod user_tests;
