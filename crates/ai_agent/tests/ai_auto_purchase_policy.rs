use chrono::{Duration, Utc};
use market_bot_ai_agent::{
    AiActionResult, AiActionType, AiError, AiScope, AutoPurchaseInput, InMemoryAiStack,
    PolicyReason, PurchasePolicy,
};
use market_bot_shared::UserId;
use uuid::Uuid;

struct PolicyFixture {
    stack: InMemoryAiStack,
    user_id: UserId,
}

impl PolicyFixture {
    fn new() -> Self {
        Self {
            stack: InMemoryAiStack::new(),
            user_id: UserId::new(),
        }
    }

    async fn authorize_auto_purchase(&self) -> market_bot_ai_agent::Authorization {
        self.stack
            .authorizations
            .authorize(
                self.user_id,
                "client-1",
                [AiScope::AutoPurchase],
                Utc::now() + Duration::hours(1),
            )
            .await
            .expect("authorization should be created")
    }

    async fn enable_auto_purchase(&self) {
        self.stack
            .authorizations
            .set_auto_purchase_enabled(self.user_id, true)
            .await
            .expect("flag should save");
    }

    async fn save_policy(&self, policy: PurchasePolicy) {
        self.stack
            .authorizations
            .save_policy(self.user_id, policy)
            .await
            .expect("policy should save");
    }
}

#[tokio::test]
async fn should_reject_auto_purchase_when_user_disabled_the_flag() {
    let fixture = PolicyFixture::new();
    let authorization = fixture.authorize_auto_purchase().await;
    fixture
        .save_policy(
            PurchasePolicy::new(["electronics"], 10_000, 20_000, 50_000, 1_500, 80, true)
                .expect("policy should be valid"),
        )
        .await;
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
            "disabled",
        ))
        .await;

    assert_eq!(result, Err(AiError::AutoPurchaseDisabled));
    let actions = fixture
        .stack
        .authorizations
        .list_actions(fixture.user_id)
        .await
        .expect("actions should load");
    assert!(actions.iter().any(|action| {
        action.action_type() == AiActionType::AutoPurchase
            && action.result() == AiActionResult::Failed
            && action.error_code() == Some("AUTO_PURCHASE_DISABLED")
            && action.order_id().is_none()
    }));
}

#[tokio::test]
async fn should_block_auto_purchase_over_max_order_minor() {
    let fixture = PolicyFixture::new();
    let authorization = fixture.authorize_auto_purchase().await;
    fixture.enable_auto_purchase().await;
    fixture
        .save_policy(
            PurchasePolicy::new(["electronics"], 7_000, 20_000, 50_000, 1_500, 80, true)
                .expect("policy should be valid"),
        )
        .await;
    let listing = fixture
        .stack
        .publish_listing("Lamp", 8_000, 400, 3, 90)
        .await
        .expect("listing should publish");

    let result = fixture
        .stack
        .agent
        .auto_purchase(auto_purchase(
            &authorization,
            &listing,
            8_000,
            400,
            "max-order",
        ))
        .await;

    assert!(matches!(
        result,
        Err(AiError::PolicyBlocked {
            reason: PolicyReason::MaxOrderExceeded
        })
    ));
    assert_blocked_audit(&fixture).await;
}

#[tokio::test]
async fn should_block_auto_purchase_over_max_daily_minor() {
    let fixture = PolicyFixture::new();
    let authorization = fixture.authorize_auto_purchase().await;
    fixture.enable_auto_purchase().await;
    fixture
        .save_policy(
            PurchasePolicy::new(["electronics"], 20_000, 5_000, 50_000, 1_500, 80, true)
                .expect("policy should be valid"),
        )
        .await;
    let listing = fixture
        .stack
        .publish_listing("Lamp", 8_000, 400, 3, 90)
        .await
        .expect("listing should publish");

    let result = fixture
        .stack
        .agent
        .auto_purchase(auto_purchase(
            &authorization,
            &listing,
            8_000,
            400,
            "max-daily",
        ))
        .await;

    assert!(matches!(
        result,
        Err(AiError::PolicyBlocked {
            reason: PolicyReason::MaxDailyExceeded
        })
    ));
    assert_blocked_audit(&fixture).await;
}

#[tokio::test]
async fn should_block_auto_purchase_over_max_monthly_minor() {
    let fixture = PolicyFixture::new();
    let authorization = fixture.authorize_auto_purchase().await;
    fixture.enable_auto_purchase().await;
    fixture
        .save_policy(
            PurchasePolicy::new(["electronics"], 20_000, 50_000, 5_000, 1_500, 80, true)
                .expect("policy should be valid"),
        )
        .await;
    let listing = fixture
        .stack
        .publish_listing("Lamp", 8_000, 400, 3, 90)
        .await
        .expect("listing should publish");

    let result = fixture
        .stack
        .agent
        .auto_purchase(auto_purchase(
            &authorization,
            &listing,
            8_000,
            400,
            "max-monthly",
        ))
        .await;

    assert!(matches!(
        result,
        Err(AiError::PolicyBlocked {
            reason: PolicyReason::MaxMonthlyExceeded
        })
    ));
    assert_blocked_audit(&fixture).await;
}

#[tokio::test]
async fn should_require_user_confirmation_when_price_exceeds_policy() {
    let fixture = PolicyFixture::new();
    let authorization = fixture.authorize_auto_purchase().await;
    fixture.enable_auto_purchase().await;
    fixture
        .save_policy(
            PurchasePolicy::new(["electronics"], 10_000, 20_000, 50_000, 1_500, 80, true)
                .expect("policy should be valid"),
        )
        .await;
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
            2_000,
            400,
            "price-change",
        ))
        .await;

    assert!(matches!(
        result,
        Err(AiError::RequiresUserConfirmation {
            reason: PolicyReason::PriceChanged
        })
    ));
    let actions = fixture
        .stack
        .authorizations
        .list_actions(fixture.user_id)
        .await
        .expect("actions should load");
    assert!(actions.iter().any(|action| {
        action.action_type() == AiActionType::AutoPurchase
            && action.result() == AiActionResult::RequiresUserConfirmation
            && action.error_code() == Some("REQUIRES_USER_CONFIRMATION")
            && action.order_id().is_none()
    }));
}

#[tokio::test]
async fn should_require_user_confirmation_when_shipping_exceeds_policy() {
    let fixture = PolicyFixture::new();
    let authorization = fixture.authorize_auto_purchase().await;
    fixture.enable_auto_purchase().await;
    fixture
        .save_policy(
            PurchasePolicy::new(["electronics"], 10_000, 20_000, 50_000, 1_500, 80, true)
                .expect("policy should be valid"),
        )
        .await;
    let listing = fixture
        .stack
        .publish_listing("Heavy Lamp", 2_500, 2_000, 3, 90)
        .await
        .expect("listing should publish");

    let result = fixture
        .stack
        .agent
        .auto_purchase(auto_purchase(
            &authorization,
            &listing,
            2_500,
            2_000,
            "shipping-over",
        ))
        .await;

    assert!(matches!(
        result,
        Err(AiError::RequiresUserConfirmation {
            reason: PolicyReason::MaxShippingExceeded
        })
    ));
    let actions = fixture
        .stack
        .authorizations
        .list_actions(fixture.user_id)
        .await
        .expect("actions should load");
    assert!(actions.iter().any(|action| {
        action.action_type() == AiActionType::AutoPurchase
            && action.result() == AiActionResult::RequiresUserConfirmation
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

async fn assert_blocked_audit(fixture: &PolicyFixture) {
    let actions = fixture
        .stack
        .authorizations
        .list_actions(fixture.user_id)
        .await
        .expect("actions should load");
    assert!(actions.iter().any(|action| {
        action.action_type() == AiActionType::AutoPurchase
            && action.result() == AiActionResult::Blocked
            && action.error_code() == Some("AUTO_PURCHASE_LIMIT_EXCEEDED")
            && action.order_id().is_none()
    }));
}
