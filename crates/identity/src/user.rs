use market_bot_shared::UserId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct User {
    id: UserId,
    email: String,
    status: UserStatus,
}

impl User {
    pub fn register(email: String) -> Result<Self, UserError> {
        let normalized_email = email.trim().to_ascii_lowercase();
        if !is_valid_email(&normalized_email) {
            return Err(UserError::InvalidEmail);
        }

        Ok(Self {
            id: UserId::new(),
            email: normalized_email,
            status: UserStatus::Active,
        })
    }

    pub const fn id(&self) -> UserId {
        self.id
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub const fn status(&self) -> UserStatus {
        self.status
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum UserStatus {
    Active,
    Suspended,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum UserError {
    #[error("email address is invalid")]
    InvalidEmail,
}

fn is_valid_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };

    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !value.chars().any(char::is_whitespace)
}
