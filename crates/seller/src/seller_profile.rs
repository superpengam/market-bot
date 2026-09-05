use market_bot_shared::{SellerId, UserId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SellerProfile {
    id: SellerId,
    owner_id: UserId,
    status: SellerStatus,
}

impl SellerProfile {
    pub fn create(owner_id: UserId) -> Self {
        Self {
            id: SellerId::new(),
            owner_id,
            status: SellerStatus::Active,
        }
    }

    pub const fn id(&self) -> SellerId {
        self.id
    }

    pub const fn owner_id(&self) -> UserId {
        self.owner_id
    }

    pub const fn status(&self) -> SellerStatus {
        self.status
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SellerStatus {
    Active,
    Suspended,
}
