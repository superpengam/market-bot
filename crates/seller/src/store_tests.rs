use market_bot_shared::{SellerId, StoreId, UserId};

use super::{Store, StoreError};

#[test]
fn should_create_a_store_with_a_slug() {
    let store = Store::create(UserId::new(), "  North Star Goods  ".to_owned())
        .expect("store name should be valid");

    assert_eq!(
        store.owner_id(),
        UserId::from_uuid(store.owner_id().as_uuid())
    );
    assert_eq!(store.slug(), "north-star-goods");
    assert_eq!(store.id(), StoreId::from_uuid(store.id().as_uuid()));
}

#[test]
fn should_reject_a_blank_store_name() {
    assert_eq!(
        Store::create(UserId::new(), "   ".to_owned()),
        Err(StoreError::BlankName)
    );
}

#[test]
fn should_create_a_seller_profile_for_a_user() {
    let user_id = UserId::new();
    let profile = super::SellerProfile::create(user_id);

    assert_eq!(profile.id(), SellerId::from_uuid(profile.id().as_uuid()));
    assert_eq!(profile.owner_id(), user_id);
}
