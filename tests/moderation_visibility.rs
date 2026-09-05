use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use http_body_util::BodyExt;
use market_bot_api::app::{AppState, build_app_with_state};
use market_bot_cart::{
    AddCartItem, CartItemSource, CartService, CartServiceError, InMemoryCartRepository,
};
use market_bot_catalog::{
    CatalogService, CreateProductCommand, InMemoryCatalogRepository, Product, ProductStatus,
    ProductType,
};
use market_bot_moderation::{
    CreateReportInput, InMemoryModerationRepository, ListingFacts, ModerationDecision,
    ModerationReason, ModerationService, SandboxContentScanner, ScanAsset, ScanVerdict,
};
use market_bot_search::{InMemorySearchRepository, SearchProductsQuery, SearchService};
use market_bot_seller::SellerStatus;
use market_bot_shared::{
    FulfillmentType, InMemoryOutboxStore, Money, OutboxStore, ProductId, ProductVariantId, UserId,
};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

type TestModerationService = ModerationService<
    InMemoryModerationRepository,
    InMemoryCatalogRepository,
    InMemorySearchRepository,
    InMemoryOutboxStore,
    SandboxContentScanner,
>;

struct VisibilityFixture {
    catalog: CatalogService<InMemoryCatalogRepository>,
    search: SearchService<InMemorySearchRepository>,
    cart: CartService<InMemoryCartRepository>,
    outbox: InMemoryOutboxStore,
    service: TestModerationService,
    admin_id: UserId,
}

impl VisibilityFixture {
    fn new() -> Self {
        let catalog_repository = InMemoryCatalogRepository::default();
        let search_repository = InMemorySearchRepository::default();
        let outbox = InMemoryOutboxStore::default();
        let service = ModerationService::new(
            InMemoryModerationRepository::default(),
            catalog_repository.clone(),
            search_repository.clone(),
            outbox.clone(),
            SandboxContentScanner,
        );

        Self {
            catalog: CatalogService::new(catalog_repository),
            search: SearchService::new(search_repository),
            cart: CartService::new(InMemoryCartRepository::default()),
            outbox,
            service,
            admin_id: UserId::new(),
        }
    }

    fn admin_reason(&self, text: &str) -> ModerationReason {
        ModerationReason::new(self.admin_id, text).expect("admin reason should be valid")
    }

    async fn create_product(&self, product_type: ProductType, title: &str) -> Product {
        self.catalog
            .create_product(CreateProductCommand {
                seller_id: market_bot_shared::SellerId::new(),
                title: title.to_owned(),
                description: format!("{title} description"),
                product_type,
                price_minor: 2_500,
                currency: "USD".to_owned(),
            })
            .await
            .expect("product should be created")
    }

    async fn record_ready_context(&self, product: &Product) {
        self.service
            .record_publish_context(
                product.id(),
                SellerStatus::Active,
                ListingFacts {
                    variant_ids: vec![ProductVariantId::new()],
                    available_stock: 4,
                },
            )
            .await
            .expect("publish context should be recorded");
    }

    async fn approve(&self, product_id: ProductId) {
        self.service
            .review_product(
                product_id,
                ModerationDecision::Approved,
                self.admin_reason("passed basic review"),
            )
            .await
            .expect("ready product should be approved");
    }

    async fn public_search_ids(&self) -> Vec<ProductId> {
        self.search
            .search(SearchProductsQuery::default())
            .await
            .expect("public search should succeed")
            .items
            .into_iter()
            .map(|item| item.product_id)
            .collect()
    }
}

fn usd(minor: i64) -> Money {
    Money::new(
        minor,
        market_bot_shared::CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("price should be valid")
}

fn cart_item(product: &Product) -> AddCartItem {
    AddCartItem {
        product_id: product.id(),
        variant_id: ProductVariantId::new(),
        title: product.title().to_owned(),
        unit_price: usd(2_500),
        quantity: 1,
        source: CartItemSource::User,
        fulfillment_type: FulfillmentType::PhysicalStandard,
    }
}

#[tokio::test]
async fn should_exclude_products_that_fail_basic_checks_from_public_search() {
    let fixture = VisibilityFixture::new();
    let not_ready = fixture
        .create_product(ProductType::PhysicalStandard, "Incomplete Lamp")
        .await;
    let ready = fixture
        .create_product(ProductType::PhysicalStandard, "Ready Lamp")
        .await;

    fixture.record_ready_context(&ready).await;
    fixture
        .service
        .record_publish_context(
            not_ready.id(),
            SellerStatus::Active,
            ListingFacts {
                variant_ids: Vec::new(),
                available_stock: 0,
            },
        )
        .await
        .expect("incomplete context should be recorded");

    fixture.approve(ready.id()).await;
    let not_ready_result = fixture
        .service
        .review_product(
            not_ready.id(),
            ModerationDecision::Approved,
            fixture.admin_reason("attempted publish"),
        )
        .await;

    assert!(not_ready_result.is_err());
    let visible = fixture.public_search_ids().await;
    assert!(visible.contains(&ready.id()));
    assert!(!visible.contains(&not_ready.id()));
}

#[tokio::test]
async fn should_reject_adding_a_suspended_product_to_cart() {
    let fixture = VisibilityFixture::new();
    let product = fixture
        .create_product(ProductType::PhysicalStandard, "Suspended Lamp")
        .await;
    fixture.record_ready_context(&product).await;
    fixture.approve(product.id()).await;

    fixture
        .service
        .review_product(
            product.id(),
            ModerationDecision::Suspended,
            fixture.admin_reason("policy violation"),
        )
        .await
        .expect("published product should be suspendable");

    let loaded = fixture
        .catalog
        .get_product(product.id())
        .await
        .expect("product should load")
        .expect("product should exist");
    assert!(!loaded.can_be_added_to_cart());
    assert!(!fixture.public_search_ids().await.contains(&product.id()));

    let cart = fixture
        .cart
        .create_cart(UserId::new())
        .await
        .expect("cart should be created");
    let can_purchase = fixture.service.ensure_can_add_to_cart(product.id()).await;
    assert!(can_purchase.is_err());

    let result = fixture
        .cart
        .add_purchasable_item(cart.id(), cart_item(&loaded), &loaded)
        .await;
    assert!(matches!(
        result,
        Err(CartServiceError::ProductNotPurchasable)
    ));
}

#[tokio::test]
async fn should_record_admin_actor_reason_and_time_on_review() {
    let fixture = VisibilityFixture::new();
    let product = fixture
        .create_product(ProductType::PhysicalStandard, "Reviewed Lamp")
        .await;
    fixture.record_ready_context(&product).await;
    let before = Utc::now();

    fixture
        .service
        .review_product(
            product.id(),
            ModerationDecision::Suspended,
            fixture.admin_reason("counterfeit listing"),
        )
        .await
        .expect("admin review should be recorded");

    let case = fixture
        .service
        .product_review_case(product.id())
        .await
        .expect("case lookup should succeed")
        .expect("review should create a case");
    let action = case
        .latest_action()
        .expect("admin review should record an action");

    assert_eq!(action.actor_id(), fixture.admin_id);
    assert_eq!(action.reason(), "counterfeit listing");
    assert_eq!(action.decision(), ModerationDecision::Suspended);
    assert!(action.acted_at() >= before);
    assert!(action.acted_at() <= Utc::now());
}

#[tokio::test]
async fn should_not_trigger_fulfillment_when_digital_file_scan_fails() {
    let fixture = VisibilityFixture::new();
    let product = fixture
        .create_product(ProductType::Digital, "Unsafe Download")
        .await;
    fixture.record_ready_context(&product).await;

    let scan = fixture
        .service
        .scan_digital_asset(ScanAsset {
            asset_id: Uuid::new_v4(),
            product_id: product.id(),
            filename: "malware.bin".to_owned(),
            content_type: "application/octet-stream".to_owned(),
            size_bytes: 128,
        })
        .await
        .expect("sandbox scanner should return a verdict");

    assert_eq!(scan.verdict(), ScanVerdict::Failed);
    assert!(
        !fixture
            .service
            .can_trigger_digital_fulfillment(product.id())
            .await
            .expect("fulfillment gate should be readable")
    );

    let approve = fixture
        .service
        .review_product(
            product.id(),
            ModerationDecision::Approved,
            fixture.admin_reason("ignore failed scan"),
        )
        .await;
    assert!(approve.is_err());
    assert!(!fixture.public_search_ids().await.contains(&product.id()));
}

#[tokio::test]
async fn should_write_outbox_events_when_a_product_is_approved_or_suspended() {
    let fixture = VisibilityFixture::new();
    let product = fixture
        .create_product(ProductType::PhysicalStandard, "Indexed Lamp")
        .await;
    fixture.record_ready_context(&product).await;
    fixture.approve(product.id()).await;
    fixture
        .service
        .review_product(
            product.id(),
            ModerationDecision::Suspended,
            fixture.admin_reason("hide from search"),
        )
        .await
        .expect("approved product should suspend");

    let events = fixture
        .outbox
        .claim_pending(16)
        .await
        .expect("outbox should be readable");
    let event_types: Vec<_> = events.iter().map(|event| event.event_type()).collect();
    assert!(event_types.contains(&"product.approved"));
    assert!(event_types.contains(&"product.suspended"));
    assert!(
        events
            .iter()
            .all(|event| !event.payload().to_string().contains("card_secret"))
    );
}

#[tokio::test]
async fn should_sanitize_card_secrets_tokens_and_addresses_from_reports() {
    let fixture = VisibilityFixture::new();
    let product = fixture
        .create_product(ProductType::PhysicalStandard, "Reported Lamp")
        .await;
    let reporter_id = UserId::new();

    let case = fixture
        .service
        .create_report(CreateReportInput {
            reporter_id,
            subject_type: market_bot_moderation::ModerationSubjectType::Product,
            subject_id: product.id().as_uuid(),
            reason_code: "prohibited_item".to_owned(),
            details: "card 4111111111111111 token tok_live_secret address: 99 Hidden Road"
                .to_owned(),
        })
        .await
        .expect("report should be created");

    let stored = case
        .latest_action()
        .map(|action| action.reason().to_owned())
        .unwrap_or_else(|| case.reason().unwrap_or_default().to_owned());
    let details = fixture
        .service
        .report_details(
            case.report_id()
                .expect("report case should have a report id"),
        )
        .await
        .expect("report details should load");

    assert!(!details.contains("4111111111111111"));
    assert!(!details.contains("tok_live_secret"));
    assert!(!details.contains("99 Hidden Road"));
    assert!(!stored.contains("4111111111111111"));
    assert!(!case.audit_text().contains("4111111111111111"));
}

#[tokio::test]
async fn should_require_separate_admin_authentication_for_review_and_suspend() {
    let product_id = Uuid::new_v4();
    let app = build_app_with_state(AppState::default());

    let unauthenticated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/products/{product_id}/reviews"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"decision":"approved","reason":"looks fine"}"#,
                ))
                .expect("unauthenticated review request should build"),
        )
        .await
        .expect("unauthenticated review request should execute");
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let forbidden = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/products/{product_id}/suspend"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer user/{}", Uuid::new_v4()))
                .body(Body::from(r#"{"reason":"not an admin"}"#))
                .expect("user suspend request should build"),
        )
        .await
        .expect("user suspend request should execute");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn should_record_admin_audit_fields_on_review_and_report_apis() {
    let state = AppState::default();
    let product = state
        .catalog
        .create_product(CreateProductCommand {
            seller_id: market_bot_shared::SellerId::new(),
            title: "Admin Lamp".to_owned(),
            description: "Reviewed through the admin API".to_owned(),
            product_type: ProductType::PhysicalStandard,
            price_minor: 1_500,
            currency: "USD".to_owned(),
        })
        .await
        .expect("product should be created");
    state
        .moderation
        .record_publish_context(
            product.id(),
            SellerStatus::Active,
            ListingFacts {
                variant_ids: vec![ProductVariantId::new()],
                available_stock: 3,
            },
        )
        .await
        .expect("publish context should be recorded");

    let admin_id = Uuid::new_v4();
    let before = Utc::now();
    let app = build_app_with_state(state.clone());
    let review = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/products/{}/reviews", product.id()))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer admin/{admin_id}"))
                .body(Body::from(
                    r#"{"decision":"suspended","reason":"trademark complaint"}"#,
                ))
                .expect("admin review request should build"),
        )
        .await
        .expect("admin review request should execute");
    assert_eq!(review.status(), StatusCode::OK);
    let review_json = json_body(review).await;
    assert_eq!(review_json["actor_id"], admin_id.to_string());
    assert_eq!(review_json["reason"], "trademark complaint");
    assert_eq!(review_json["decision"], "suspended");
    let acted_at = review_json["acted_at"]
        .as_str()
        .expect("acted_at should be present");
    let parsed = chrono::DateTime::parse_from_rfc3339(acted_at)
        .expect("acted_at should be RFC3339")
        .with_timezone(&Utc);
    assert!(parsed >= before);

    let reporter_id = Uuid::new_v4();
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reports")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer user/{reporter_id}"))
                .body(Body::from(format!(
                    r#"{{"subject_type":"product","subject_id":"{}","reason_code":"spam","details":"repeated listing"}}"#,
                    product.id()
                )))
                .expect("create report request should build"),
        )
        .await
        .expect("create report request should execute");
    assert_eq!(created.status(), StatusCode::OK);
    let created_json = json_body(created).await;
    assert_eq!(created_json["status"], "in_review");
    let report_id = created_json["report_id"]
        .as_str()
        .expect("report_id should be present");

    let resolved = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/reports/{report_id}/resolve"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer admin/{admin_id}"))
                .body(Body::from(
                    r#"{"decision":"rejected","reason":"no violation found"}"#,
                ))
                .expect("resolve report request should build"),
        )
        .await
        .expect("resolve report request should execute");
    assert_eq!(resolved.status(), StatusCode::OK);
    let resolved_json = json_body(resolved).await;
    assert_eq!(resolved_json["actor_id"], admin_id.to_string());
    assert_eq!(resolved_json["reason"], "no violation found");
    assert!(resolved_json["acted_at"].as_str().is_some());
}

#[tokio::test]
async fn should_republish_a_suspended_product_when_approved() {
    let fixture = VisibilityFixture::new();
    let product = fixture
        .create_product(ProductType::PhysicalStandard, "Restored Lamp")
        .await;
    fixture.record_ready_context(&product).await;
    fixture.approve(product.id()).await;
    fixture
        .service
        .review_product(
            product.id(),
            ModerationDecision::Suspended,
            fixture.admin_reason("temporary takedown"),
        )
        .await
        .expect("published product should suspend");

    let suspended = fixture
        .catalog
        .get_product(product.id())
        .await
        .expect("product should load")
        .expect("product should exist");
    assert_eq!(suspended.status(), ProductStatus::Suspended);
    assert!(!fixture.public_search_ids().await.contains(&product.id()));

    fixture.approve(product.id()).await;

    let restored = fixture
        .catalog
        .get_product(product.id())
        .await
        .expect("product should load")
        .expect("product should exist");
    assert_eq!(restored.status(), ProductStatus::Published);
    assert!(restored.can_be_added_to_cart());
    assert!(fixture.public_search_ids().await.contains(&product.id()));

    let events = fixture
        .outbox
        .claim_pending(16)
        .await
        .expect("outbox should be readable");
    let approved_events: Vec<_> = events
        .iter()
        .filter(|event| event.event_type() == "product.approved")
        .collect();
    assert_eq!(approved_events.len(), 2);
    assert!(events.iter().any(|event| {
        event.event_type() == "product.approved" && event.payload()["decision"] == "approved"
    }));
}

#[tokio::test]
async fn should_sanitize_review_reason_in_case_http_and_outbox() {
    let fixture = VisibilityFixture::new();
    let product = fixture
        .create_product(ProductType::PhysicalStandard, "Secret Lamp")
        .await;
    fixture.record_ready_context(&product).await;
    let secret_reason = "card 4111111111111111 token tok_live_secret address: 99 Hidden Road";

    fixture
        .service
        .review_product(
            product.id(),
            ModerationDecision::Suspended,
            fixture.admin_reason(secret_reason),
        )
        .await
        .expect("review should accept a secret-bearing reason after sanitizing");

    let case = fixture
        .service
        .product_review_case(product.id())
        .await
        .expect("case lookup should succeed")
        .expect("review should create a case");
    let stored = case.reason().unwrap_or_default();
    assert!(!stored.contains("4111111111111111"));
    assert!(!stored.contains("tok_live_secret"));
    assert!(!stored.contains("99 Hidden Road"));

    let events = fixture
        .outbox
        .claim_pending(16)
        .await
        .expect("outbox should be readable");
    let payload = events
        .iter()
        .find(|event| event.event_type() == "product.suspended")
        .expect("suspend should write an outbox event")
        .payload()
        .to_string();
    assert!(!payload.contains("4111111111111111"));
    assert!(!payload.contains("tok_live_secret"));
    assert!(!payload.contains("99 Hidden Road"));

    let state = AppState::default();
    let http_product = state
        .catalog
        .create_product(CreateProductCommand {
            seller_id: market_bot_shared::SellerId::new(),
            title: "HTTP Secret Lamp".to_owned(),
            description: "Reviewed with secrets".to_owned(),
            product_type: ProductType::PhysicalStandard,
            price_minor: 1_500,
            currency: "USD".to_owned(),
        })
        .await
        .expect("product should be created");
    state
        .moderation
        .record_publish_context(
            http_product.id(),
            SellerStatus::Active,
            ListingFacts {
                variant_ids: vec![ProductVariantId::new()],
                available_stock: 3,
            },
        )
        .await
        .expect("publish context should be recorded");
    let app = build_app_with_state(state);
    let review = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/admin/products/{}/reviews",
                    http_product.id()
                ))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer admin/{}", Uuid::new_v4()))
                .body(Body::from(format!(
                    r#"{{"decision":"suspended","reason":"{secret_reason}"}}"#
                )))
                .expect("admin review request should build"),
        )
        .await
        .expect("admin review request should execute");
    assert_eq!(review.status(), StatusCode::OK);
    let review_json = json_body(review).await;
    let http_reason = review_json["reason"]
        .as_str()
        .expect("reason should be present");
    assert!(!http_reason.contains("4111111111111111"));
    assert!(!http_reason.contains("tok_live_secret"));
    assert!(!http_reason.contains("99 Hidden Road"));
}

#[tokio::test]
async fn should_replay_moderation_writes_for_the_same_idempotency_key() {
    let state = AppState::default();
    let product = state
        .catalog
        .create_product(CreateProductCommand {
            seller_id: market_bot_shared::SellerId::new(),
            title: "Idempotent Lamp".to_owned(),
            description: "Reviewed twice with the same key".to_owned(),
            product_type: ProductType::PhysicalStandard,
            price_minor: 1_500,
            currency: "USD".to_owned(),
        })
        .await
        .expect("product should be created");
    state
        .moderation
        .record_publish_context(
            product.id(),
            SellerStatus::Active,
            ListingFacts {
                variant_ids: vec![ProductVariantId::new()],
                available_stock: 3,
            },
        )
        .await
        .expect("publish context should be recorded");

    let admin_id = Uuid::new_v4();
    let reporter_id = Uuid::new_v4();
    let app = build_app_with_state(state.clone());

    let review_one = send_json(
        app.clone(),
        "POST",
        &format!("/api/v1/admin/products/{}/reviews", product.id()),
        Some(&format!("Bearer admin/{admin_id}")),
        Some("review-key-1"),
        r#"{"decision":"approved","reason":"ready to list"}"#,
    )
    .await;
    let review_two = send_json(
        app.clone(),
        "POST",
        &format!("/api/v1/admin/products/{}/reviews", product.id()),
        Some(&format!("Bearer admin/{admin_id}")),
        Some("review-key-1"),
        r#"{"decision":"approved","reason":"ready to list"}"#,
    )
    .await;
    assert_eq!(review_one.status(), StatusCode::OK);
    assert_eq!(review_two.status(), StatusCode::OK);
    let review_one_json = json_body(review_one).await;
    let review_two_json = json_body(review_two).await;
    assert_eq!(review_one_json, review_two_json);
    let review_case = state
        .moderation
        .product_review_case(product.id())
        .await
        .expect("case lookup should succeed")
        .expect("review should create a case");
    assert_eq!(review_case.actions().len(), 1);

    let suspend_one = send_json(
        app.clone(),
        "POST",
        &format!("/api/v1/admin/products/{}/suspend", product.id()),
        Some(&format!("Bearer admin/{admin_id}")),
        Some("suspend-key-1"),
        r#"{"reason":"policy hold"}"#,
    )
    .await;
    let suspend_two = send_json(
        app.clone(),
        "POST",
        &format!("/api/v1/admin/products/{}/suspend", product.id()),
        Some(&format!("Bearer admin/{admin_id}")),
        Some("suspend-key-1"),
        r#"{"reason":"policy hold"}"#,
    )
    .await;
    assert_eq!(suspend_one.status(), StatusCode::OK);
    assert_eq!(suspend_two.status(), StatusCode::OK);
    assert_eq!(json_body(suspend_one).await, json_body(suspend_two).await);
    let suspend_case = state
        .moderation
        .product_review_case(product.id())
        .await
        .expect("case lookup should succeed")
        .expect("suspend should reuse the product case");
    assert_eq!(suspend_case.actions().len(), 2);

    let report_one = send_json(
        app.clone(),
        "POST",
        "/api/v1/reports",
        Some(&format!("Bearer user/{reporter_id}")),
        Some("report-key-1"),
        &format!(
            r#"{{"subject_type":"product","subject_id":"{}","reason_code":"spam","details":"duplicate listing"}}"#,
            product.id()
        ),
    )
    .await;
    let report_two = send_json(
        app.clone(),
        "POST",
        "/api/v1/reports",
        Some(&format!("Bearer user/{reporter_id}")),
        Some("report-key-1"),
        &format!(
            r#"{{"subject_type":"product","subject_id":"{}","reason_code":"spam","details":"duplicate listing"}}"#,
            product.id()
        ),
    )
    .await;
    assert_eq!(report_one.status(), StatusCode::OK);
    assert_eq!(report_two.status(), StatusCode::OK);
    let report_one_json = json_body(report_one).await;
    let report_two_json = json_body(report_two).await;
    assert_eq!(report_one_json, report_two_json);
    let report_id = report_one_json["report_id"]
        .as_str()
        .expect("report_id should be present");

    let resolve_one = send_json(
        app.clone(),
        "POST",
        &format!("/api/v1/admin/reports/{report_id}/resolve"),
        Some(&format!("Bearer admin/{admin_id}")),
        Some("resolve-key-1"),
        r#"{"decision":"rejected","reason":"no violation found"}"#,
    )
    .await;
    let resolve_two = send_json(
        app,
        "POST",
        &format!("/api/v1/admin/reports/{report_id}/resolve"),
        Some(&format!("Bearer admin/{admin_id}")),
        Some("resolve-key-1"),
        r#"{"decision":"rejected","reason":"no violation found"}"#,
    )
    .await;
    assert_eq!(resolve_one.status(), StatusCode::OK);
    assert_eq!(resolve_two.status(), StatusCode::OK);
    assert_eq!(json_body(resolve_one).await, json_body(resolve_two).await);
}

async fn send_json(
    app: axum::Router,
    method: &str,
    uri: &str,
    authorization: Option<&str>,
    idempotency_key: Option<&str>,
    body: &str,
) -> axum::http::Response<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(authorization) = authorization {
        builder = builder.header("authorization", authorization);
    }
    if let Some(idempotency_key) = idempotency_key {
        builder = builder.header("Idempotency-Key", idempotency_key);
    }
    app.oneshot(
        builder
            .body(Body::from(body.to_owned()))
            .expect("request should build"),
    )
    .await
    .expect("request should execute")
}

async fn json_body(response: axum::http::Response<Body>) -> Value {
    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should be readable")
        .to_bytes();
    serde_json::from_slice(&body).expect("response should be JSON")
}
