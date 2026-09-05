use market_bot_shared::{StoreId, UserId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Store {
    id: StoreId,
    owner_id: UserId,
    name: String,
    slug: String,
}

impl Store {
    pub fn create(owner_id: UserId, name: String) -> Result<Self, StoreError> {
        let name = name.trim().to_owned();
        if name.is_empty() {
            return Err(StoreError::BlankName);
        }

        let slug = slugify(&name);
        if slug.is_empty() {
            return Err(StoreError::InvalidName);
        }

        Ok(Self {
            id: StoreId::new(),
            owner_id,
            name,
            slug,
        })
    }

    pub const fn id(&self) -> StoreId {
        self.id
    }

    pub const fn owner_id(&self) -> UserId {
        self.owner_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum StoreError {
    #[error("store name cannot be blank")]
    BlankName,
    #[error("store name cannot produce a valid slug")]
    InvalidName,
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = false;

    for character in value.chars() {
        if character.is_alphanumeric() {
            slug.extend(character.to_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator && !slug.is_empty() {
            slug.push('-');
            previous_was_separator = true;
        }
    }

    slug.trim_end_matches('-').to_owned()
}
