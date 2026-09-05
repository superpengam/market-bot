use chrono::{Duration, Utc};
use market_bot_ai_agent::{
    AddToCartInput, AiActionResult, AiActionType, AiError, AiScope, AutoPurchaseInput,
    CartItemSource, InMemoryAiStack, PurchasePolicy, SearchProductsInput,
};
use market_bot_shared::UserId;
use uuid::Uuid;

struct AuthFixture {
    stack: InMemoryAiStack,
    user_id: UserId,
}

impl AuthFixture {
    fn new() -> Self {
        Self {
            stack: InMemoryAiStack::new(),
            user_id: UserId::new(),
        }
    }

    async fn authorize(&self, scopes: &[AiScope]) -> market_bot_ai_agent::Authorization {
        self.stack
            .authorizations
            .authorize(
                self.user_id,
                "client-1",
                scopes.iter().copied(),
                Utc::now() + Duration::hours(1),
            )
            .await
            .expect("authorization should be created")
    }

    fn policy() -> PurchasePolicy {
        PurchasePolicy::new(["electronics"], 10_000, 20_000, 50_000, 1_500, 80, true)
            .expect("policy should be valid")
    }
}

#[tokio::test]
async fn should_reject_add_to_cart_without_cart_write_scope() {
    let fixture = AuthFixture::new();
    let authorization = fixture
        .authorize(&[AiScope::CartRead, AiScope::CatalogRead])
        .await;
    let listing = fixture
        .stack
        .publish_listing("Lamp", 2_500, 400, 3, 90)
        .await
        .expect("listing should publish");
    let cart = fixture
        .stack
        .agent
        .cart_service()
        .create_cart(fixture.user_id)
        .await
        .expect("cart should be created");

    let result = fixture
        .stack
        .agent
        .add_to_cart(AddToCartInput {
            authorization_id: authorization.id(),
            cart_id: cart.id(),
            product_id: listing.product_id,
            variant_id: listing.variant_id,
            quantity: 1,
            request_id: Uuid::new_v4(),
        })
        .await;

    assert!(matches!(
        result,
        Err(AiError::MissingScope {
            required: AiScope::CartWrite
        })
    ));
    assert_action(
        &fixture,
        AiActionType::AddToCart,
        AiActionResult::Failed,
        Some("MISSING_SCOPE"),
    )
    .await;
}

#[tokio::test]
async fn should_reject_auto_purchase_without_auto_purchase_scope() {
    let fixture = AuthFixture::new();
    let authorization = fixture
        .authorize(&[AiScope::OrderCreate, AiScope::CartWrite])
        .await;
    fixture
        .stack
        .authorizations
        .save_policy(fixture.user_id, AuthFixture::policy())
        .await
        .expect("policy should save");
    fixture
        .stack
        .authorizations
        .set_auto_purchase_enabled(fixture.user_id, true)
        .await
        .expect("flag should save");
    let listing = fixture
        .stack
        .publish_listing("Lamp", 2_500, 400, 3, 90)
        .await
        .expect("listing should publish");

    let result = fixture
        .stack
        .agent
        .auto_purchase(auto_purchase(
            &authorization,
            &listing,
            2_500,
            400,
            "no-scope",
        ))
        .await;

    assert!(matches!(
        result,
        Err(AiError::MissingScope {
            required: AiScope::AutoPurchase
        })
    ));
    assert_action(
        &fixture,
        AiActionType::AutoPurchase,
        AiActionResult::Failed,
        Some("MISSING_SCOPE"),
    )
    .await;
}

#[tokio::test]
async fn should_not_let_order_create_inherit_auto_purchase() {
    let fixture = AuthFixture::new();
    let authorization = fixture.authorize(&[AiScope::OrderCreate]).await;
    fixture
        .stack
        .authorizations
        .save_policy(fixture.user_id, AuthFixture::policy())
        .await
        .expect("policy should save");
    fixture
        .stack
        .authorizations
        .set_auto_purchase_enabled(fixture.user_id, true)
        .await
        .expect("flag should save");
    let listing = fixture
        .stack
        .publish_listing("Lamp", 2_500, 400, 3, 90)
        .await
        .expect("listing should publish");

    let result = fixture
        .stack
        .agent
        .auto_purchase(auto_purchase(
            &authorization,
            &listing,
            2_500,
            400,
            "order-create-only",
        ))
        .await;

    assert!(matches!(
        result,
        Err(AiError::MissingScope {
            required: AiScope::AutoPurchase
        })
    ));
}

#[tokio::test]
async fn should_reject_expired_authorization() {
    let fixture = AuthFixture::new();
    let authorization = fixture
        .stack
        .authorizations
        .authorize(
            fixture.user_id,
            "client-1",
            [AiScope::CartWrite, AiScope::AutoPurchase],
            Utc::now() - Duration::minutes(5),
        )
        .await
        .expect("expired grant can be stored");
    let listing = fixture
        .stack
        .publish_listing("Lamp", 2_500, 400, 3, 90)
        .await
        .expect("listing should publish");
    let cart = fixture
        .stack
        .agent
        .cart_service()
        .create_cart(fixture.user_id)
        .await
        .expect("cart should be created");

    let add = fixture
        .stack
        .agent
        .add_to_cart(AddToCartInput {
            authorization_id: authorization.id(),
            cart_id: cart.id(),
            product_id: listing.product_id,
            variant_id: listing.variant_id,
            quantity: 1,
            request_id: Uuid::new_v4(),
        })
        .await;
    let purchase = fixture
        .stack
        .agent
        .auto_purchase(auto_purchase(
            &authorization,
            &listing,
            2_500,
            400,
            "expired",
        ))
        .await;

    assert_eq!(add, Err(AiError::AuthorizationExpired));
    assert_eq!(purchase, Err(AiError::AuthorizationExpired));
    assert_action(
        &fixture,
        AiActionType::AddToCart,
        AiActionResult::Failed,
        Some("AI_AUTHORIZATION_EXPIRED"),
    )
    .await;
}

#[tokio::test]
async fn should_write_an_audit_action_for_every_ai_operation() {
    let fixture = AuthFixture::new();
    let authorization = fixture
        .authorize(&[
            AiScope::CatalogRead,
            AiScope::CartWrite,
            AiScope::AutoPurchase,
        ])
        .await;
    fixture
        .stack
        .authorizations
        .save_policy(fixture.user_id, AuthFixture::policy())
        .await
        .expect("policy should save");
    fixture
        .stack
        .authorizations
        .set_auto_purchase_enabled(fixture.user_id, true)
        .await
        .expect("flag should save");
    let listing = fixture
        .stack
        .publish_listing("Lamp", 2_500, 400, 3, 90)
        .await
        .expect("listing should publish");
    let cart = fixture
        .stack
        .agent
        .cart_service()
        .create_cart(fixture.user_id)
        .await
        .expect("cart should be created");

    fixture
        .stack
        .agent
        .search_products(SearchProductsInput {
            authorization_id: authorization.id(),
            query: Some("Lamp".to_owned()),
            category_id: None,
            request_id: Uuid::new_v4(),
        })
        .await
        .expect("search should succeed");
    let item = fixture
        .stack
        .agent
        .add_to_cart(AddToCartInput {
            authorization_id: authorization.id(),
            cart_id: cart.id(),
            product_id: listing.product_id,
            variant_id: listing.variant_id,
            quantity: 1,
            request_id: Uuid::new_v4(),
        })
        .await
        .expect("add to cart should succeed");
    assert_eq!(item.source(), CartItemSource::Ai);
    let order = fixture
        .stack
        .agent
        .auto_purchase(auto_purchase(&authorization, &listing, 2_500, 400, "ok"))
        .await
        .expect("auto purchase should succeed");

    let actions = fixture
        .stack
        .authorizations
        .list_actions(fixture.user_id)
        .await
        .expect("actions should load");
    let types: Vec<_> = actions.iter().map(|action| action.action_type()).collect();
    assert!(types.contains(&AiActionType::Authorize));
    assert!(types.contains(&AiActionType::SearchProducts));
    assert!(types.contains(&AiActionType::AddToCart));
    assert!(types.contains(&AiActionType::AutoPurchase));
    assert!(actions.iter().any(|action| {
        action.action_type() == AiActionType::AutoPurchase
            && action.result() == AiActionResult::Succeeded
            && action.order_id() == Some(order.id())
            && action.request_id() != Uuid::nil()
    }));
}

fn auto_purchase(
    authorization: &market_bot_ai_agent::Authorization,
    listing: &market_bot_ai_agent::PublishedListing,
    quoted_unit_price_minor: i64,
    quoted_shipping_minor: i64,
    key: &str,
) -> AutoPurchaseInput {
    AutoPurchaseInput {
        authorization_id: authorization.id(),
        product_id: listing.product_id,
        variant_id: listing.variant_id,
        quantity: 1,
        quoted_unit_price_minor,
        quoted_shipping_minor,
        idempotency_key: key.to_owned(),
        request_id: Uuid::new_v4(),
        now: Utc::now(),
    }
}

async fn assert_action(
    fixture: &AuthFixture,
    action_type: AiActionType,
    result: AiActionResult,
    error_code: Option<&str>,
) {
    let actions = fixture
        .stack
        .authorizations
        .list_actions(fixture.user_id)
        .await
        .expect("actions should load");
    assert!(
        actions.iter().any(|action| {
            action.action_type() == action_type
                && action.result() == result
                && action.error_code() == error_code
        }),
        "missing audit action {action_type:?} {result:?} {error_code:?} in {actions:?}"
    );
}
