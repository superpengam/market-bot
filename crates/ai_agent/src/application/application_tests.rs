use chrono::{Duration, Utc};
use market_bot_catalog::{CatalogRepository, CreateProductCommand, Product, ProductType};
use market_bot_shared::{CurrencyCode, Money, ProductId, ProductVariantId, SellerId, UserId};
use uuid::Uuid;

use crate::application::{AddToCartInput, AutoPurchaseInput, SearchProductsInput};
use crate::domain::{AiActionResult, AiActionType, AiScope, PolicyReason, PurchasePolicy};
use crate::{AiError, CartItemSource, CatalogListingExtras, InMemoryAiStack};

struct Fixture {
    stack: InMemoryAiStack,
    user_id: UserId,
}

impl Fixture {
    fn new() -> Self {
        Self {
            stack: InMemoryAiStack::new(),
            user_id: UserId::new(),
        }
    }

    async fn authorize(&self, scopes: &[AiScope], expires_in_hours: i64) -> crate::Authorization {
        self.stack
            .authorizations
            .authorize(
                self.user_id,
                "client-1",
                scopes.iter().copied(),
                Utc::now() + Duration::hours(expires_in_hours),
            )
            .await
            .expect("authorization should be created")
    }

    async fn save_policy(&self, policy: PurchasePolicy) {
        self.stack
            .authorizations
            .save_policy(self.user_id, policy)
            .await
            .expect("policy should save");
    }

    async fn enable_auto_purchase(&self) {
        self.stack
            .authorizations
            .set_auto_purchase_enabled(self.user_id, true)
            .await
            .expect("auto-purchase flag should save");
    }

    async fn publish_listing(
        &self,
        title: &str,
        price_minor: i64,
        shipping_minor: i64,
        stock: u64,
        seller_score: i32,
    ) -> (Product, ProductVariantId) {
        let mut product = self
            .stack
            .catalog
            .create_product(CreateProductCommand {
                seller_id: SellerId::new(),
                title: title.to_owned(),
                description: format!("{title} description"),
                product_type: ProductType::PhysicalStandard,
                price_minor,
                currency: "USD".to_owned(),
            })
            .await
            .expect("product should be created");
        product
            .submit_for_review()
            .expect("draft should enter review");
        product.publish().expect("reviewed product should publish");
        self.stack
            .catalog_repo
            .save_product(product.clone())
            .await
            .expect("published product should persist");

        let variant_id = ProductVariantId::new();
        self.stack
            .catalog
            .initialize_inventory(variant_id, stock)
            .await
            .expect("inventory should initialize");
        self.stack
            .facts
            .register_extras(CatalogListingExtras {
                product_id: product.id(),
                variant_id,
                category_id: "electronics".to_owned(),
                seller_score,
                shipping_fee: usd(shipping_minor),
                tax: usd(0),
            })
            .await
            .expect("listing extras should register");
        (product, variant_id)
    }

    fn policy(&self) -> PurchasePolicy {
        PurchasePolicy::new(["electronics"], 10_000, 20_000, 50_000, 1_500, 80, true)
            .expect("policy should be valid")
    }
}

fn usd(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("money should be valid")
}

#[tokio::test]
async fn should_reject_add_to_cart_without_cart_write_scope() {
    let fixture = Fixture::new();
    let authorization = fixture.authorize(&[AiScope::CartRead], 1).await;
    let (product, variant_id) = fixture.publish_listing("Lamp", 2_500, 400, 3, 90).await;
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
            product_id: product.id(),
            variant_id,
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
    let actions = fixture
        .stack
        .authorizations
        .list_actions(fixture.user_id)
        .await
        .expect("actions should load");
    assert!(
        actions
            .iter()
            .any(|action| action.action_type() == AiActionType::AddToCart
                && action.result() == AiActionResult::Failed)
    );
}

#[tokio::test]
async fn should_reject_auto_purchase_without_auto_purchase_scope() {
    let fixture = Fixture::new();
    let authorization = fixture
        .authorize(&[AiScope::OrderCreate, AiScope::CartWrite], 1)
        .await;
    fixture.save_policy(fixture.policy()).await;
    fixture.enable_auto_purchase().await;
    let (product, variant_id) = fixture.publish_listing("Lamp", 2_500, 400, 3, 90).await;

    let result = fixture
        .stack
        .agent
        .auto_purchase(AutoPurchaseInput {
            authorization_id: authorization.id(),
            product_id: product.id(),
            variant_id,
            quantity: 1,
            quoted_unit_price_minor: 2_500,
            quoted_shipping_minor: 400,
            idempotency_key: "order-1".to_owned(),
            request_id: Uuid::new_v4(),
            now: Utc::now(),
        })
        .await;

    assert!(matches!(
        result,
        Err(AiError::MissingScope {
            required: AiScope::AutoPurchase
        })
    ));
}

#[tokio::test]
async fn should_not_let_order_create_inherit_auto_purchase() {
    let fixture = Fixture::new();
    let authorization = fixture.authorize(&[AiScope::OrderCreate], 1).await;
    fixture.save_policy(fixture.policy()).await;
    fixture.enable_auto_purchase().await;
    let (product, variant_id) = fixture.publish_listing("Lamp", 2_500, 400, 3, 90).await;

    let result = fixture
        .stack
        .agent
        .auto_purchase(AutoPurchaseInput {
            authorization_id: authorization.id(),
            product_id: product.id(),
            variant_id,
            quantity: 1,
            quoted_unit_price_minor: 2_500,
            quoted_shipping_minor: 400,
            idempotency_key: "order-inherit".to_owned(),
            request_id: Uuid::new_v4(),
            now: Utc::now(),
        })
        .await;

    assert!(matches!(
        result,
        Err(AiError::MissingScope {
            required: AiScope::AutoPurchase
        })
    ));
}

#[tokio::test]
async fn should_reject_auto_purchase_when_user_disabled_the_flag() {
    let fixture = Fixture::new();
    let authorization = fixture.authorize(&[AiScope::AutoPurchase], 1).await;
    fixture.save_policy(fixture.policy()).await;
    let (product, variant_id) = fixture.publish_listing("Lamp", 2_500, 400, 3, 90).await;

    let result = fixture
        .stack
        .agent
        .auto_purchase(AutoPurchaseInput {
            authorization_id: authorization.id(),
            product_id: product.id(),
            variant_id,
            quantity: 1,
            quoted_unit_price_minor: 2_500,
            quoted_shipping_minor: 400,
            idempotency_key: "order-disabled".to_owned(),
            request_id: Uuid::new_v4(),
            now: Utc::now(),
        })
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
            && action.error_code() == Some("AUTO_PURCHASE_DISABLED")
    }));
}

#[tokio::test]
async fn should_block_auto_purchase_over_order_daily_and_monthly_caps() {
    let fixture = Fixture::new();
    let authorization = fixture.authorize(&[AiScope::AutoPurchase], 1).await;
    fixture.enable_auto_purchase().await;
    let (product, variant_id) = fixture.publish_listing("Lamp", 8_000, 400, 5, 90).await;

    fixture
        .save_policy(
            PurchasePolicy::new(["electronics"], 7_000, 20_000, 50_000, 1_500, 80, true)
                .expect("policy should be valid"),
        )
        .await;
    let over_order = fixture
        .stack
        .agent
        .auto_purchase(purchase(
            &authorization,
            product.id(),
            variant_id,
            "cap-order",
        ))
        .await;
    assert!(matches!(
        over_order,
        Err(AiError::PolicyBlocked {
            reason: PolicyReason::MaxOrderExceeded
        })
    ));

    fixture
        .save_policy(
            PurchasePolicy::new(["electronics"], 20_000, 5_000, 50_000, 1_500, 80, true)
                .expect("policy should be valid"),
        )
        .await;
    let over_daily = fixture
        .stack
        .agent
        .auto_purchase(purchase(
            &authorization,
            product.id(),
            variant_id,
            "cap-daily",
        ))
        .await;
    assert!(matches!(
        over_daily,
        Err(AiError::PolicyBlocked {
            reason: PolicyReason::MaxDailyExceeded
        })
    ));

    fixture
        .save_policy(
            PurchasePolicy::new(["electronics"], 20_000, 50_000, 5_000, 1_500, 80, true)
                .expect("policy should be valid"),
        )
        .await;
    let over_monthly = fixture
        .stack
        .agent
        .auto_purchase(purchase(
            &authorization,
            product.id(),
            variant_id,
            "cap-monthly",
        ))
        .await;
    assert!(matches!(
        over_monthly,
        Err(AiError::PolicyBlocked {
            reason: PolicyReason::MaxMonthlyExceeded
        })
    ));
}

#[tokio::test]
async fn should_require_user_confirmation_when_price_or_shipping_exceeds_policy() {
    let fixture = Fixture::new();
    let authorization = fixture.authorize(&[AiScope::AutoPurchase], 1).await;
    fixture.save_policy(fixture.policy()).await;
    fixture.enable_auto_purchase().await;
    let (product, variant_id) = fixture.publish_listing("Lamp", 2_500, 400, 3, 90).await;

    let price_changed = fixture
        .stack
        .agent
        .auto_purchase(AutoPurchaseInput {
            authorization_id: authorization.id(),
            product_id: product.id(),
            variant_id,
            quantity: 1,
            quoted_unit_price_minor: 2_000,
            quoted_shipping_minor: 400,
            idempotency_key: "price-change".to_owned(),
            request_id: Uuid::new_v4(),
            now: Utc::now(),
        })
        .await;
    assert!(matches!(
        price_changed,
        Err(AiError::RequiresUserConfirmation {
            reason: PolicyReason::PriceChanged
        })
    ));

    let (expensive_ship, ship_variant) = fixture
        .publish_listing("Heavy Lamp", 2_500, 2_000, 3, 90)
        .await;
    let shipping_over = fixture
        .stack
        .agent
        .auto_purchase(AutoPurchaseInput {
            authorization_id: authorization.id(),
            product_id: expensive_ship.id(),
            variant_id: ship_variant,
            quantity: 1,
            quoted_unit_price_minor: 2_500,
            quoted_shipping_minor: 2_000,
            idempotency_key: "shipping-over".to_owned(),
            request_id: Uuid::new_v4(),
            now: Utc::now(),
        })
        .await;
    assert!(matches!(
        shipping_over,
        Err(AiError::RequiresUserConfirmation {
            reason: PolicyReason::MaxShippingExceeded
        })
    ));
}

#[tokio::test]
async fn should_reject_expired_authorization() {
    let fixture = Fixture::new();
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
    let (product, variant_id) = fixture.publish_listing("Lamp", 2_500, 400, 3, 90).await;
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
            product_id: product.id(),
            variant_id,
            quantity: 1,
            request_id: Uuid::new_v4(),
        })
        .await;
    assert_eq!(add, Err(AiError::AuthorizationExpired));
}

#[tokio::test]
async fn should_write_an_audit_action_for_every_ai_operation() {
    let fixture = Fixture::new();
    let authorization = fixture
        .authorize(
            &[
                AiScope::CatalogRead,
                AiScope::CartWrite,
                AiScope::AutoPurchase,
            ],
            1,
        )
        .await;
    fixture.save_policy(fixture.policy()).await;
    fixture.enable_auto_purchase().await;
    let (product, variant_id) = fixture.publish_listing("Lamp", 2_500, 400, 3, 90).await;
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
            product_id: product.id(),
            variant_id,
            quantity: 1,
            request_id: Uuid::new_v4(),
        })
        .await
        .expect("add to cart should succeed");
    assert_eq!(item.source(), CartItemSource::Ai);

    let order = fixture
        .stack
        .agent
        .auto_purchase(AutoPurchaseInput {
            authorization_id: authorization.id(),
            product_id: product.id(),
            variant_id,
            quantity: 1,
            quoted_unit_price_minor: 2_500,
            quoted_shipping_minor: 400,
            idempotency_key: "audit-ok".to_owned(),
            request_id: Uuid::new_v4(),
            now: Utc::now(),
        })
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
    assert!(
        actions
            .iter()
            .any(|action| action.action_type() == AiActionType::AutoPurchase
                && action.order_id() == Some(order.id())
                && action.result() == AiActionResult::Succeeded)
    );
}

#[tokio::test]
async fn should_refuse_unpublished_products_for_ai_add_to_cart() {
    let fixture = Fixture::new();
    let authorization = fixture.authorize(&[AiScope::CartWrite], 1).await;
    let product = fixture
        .stack
        .catalog
        .create_product(CreateProductCommand {
            seller_id: SellerId::new(),
            title: "Draft Lamp".to_owned(),
            description: "Not public".to_owned(),
            product_type: ProductType::PhysicalStandard,
            price_minor: 2_500,
            currency: "USD".to_owned(),
        })
        .await
        .expect("draft product should be created");
    let variant_id = ProductVariantId::new();
    fixture
        .stack
        .catalog
        .initialize_inventory(variant_id, 2)
        .await
        .expect("inventory should initialize");
    fixture
        .stack
        .facts
        .register_extras(CatalogListingExtras {
            product_id: product.id(),
            variant_id,
            category_id: "electronics".to_owned(),
            seller_score: 90,
            shipping_fee: usd(400),
            tax: usd(0),
        })
        .await
        .expect("extras should register");
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
            product_id: product.id(),
            variant_id,
            quantity: 1,
            request_id: Uuid::new_v4(),
        })
        .await;

    assert_eq!(result, Err(AiError::ProductNotPurchasable));
}

fn purchase(
    authorization: &crate::Authorization,
    product_id: ProductId,
    variant_id: ProductVariantId,
    key: &str,
) -> AutoPurchaseInput {
    AutoPurchaseInput {
        authorization_id: authorization.id(),
        product_id,
        variant_id,
        quantity: 1,
        quoted_unit_price_minor: 8_000,
        quoted_shipping_minor: 400,
        idempotency_key: key.to_owned(),
        request_id: Uuid::new_v4(),
        now: Utc::now(),
    }
}
