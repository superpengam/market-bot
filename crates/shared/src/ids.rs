use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
        )]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self::from_uuid(value)
            }
        }

        impl From<$name> for Uuid {
            fn from(value: $name) -> Self {
                value.as_uuid()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

define_id!(UserId);
define_id!(SellerId);
define_id!(StoreId);
define_id!(ProductId);
define_id!(ProductVariantId);
define_id!(CartId);
define_id!(CartItemId);
define_id!(OrderId);
define_id!(OrderItemId);
define_id!(PaymentId);
define_id!(FulfillmentId);
define_id!(DigitalAssetId);
define_id!(ShipmentId);
define_id!(SettlementId);
define_id!(AiAuthorizationId);
define_id!(AiActionId);
define_id!(StockReservationId);
