# Market Bot MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在保持 Rust 模块化单体边界的前提下，交付 Market Bot 首期可测试交易闭环：用户和卖家、数字商品、标准实物商品、购物车、订单、支付沙盒、数字交付、物流状态、AI 搜索与受限自动下单。

**Architecture:** 后端使用 Rust stable 模块化单体，API 和 Worker 作为两个可独立运行的应用，共享 Cargo workspace 和领域模块。PostgreSQL 保存交易事实，Redis 处理缓存和限流，OpenSearch 处理检索，事务消息外发机制连接消息队列和异步 Worker。前端使用 Next.js，通过 `/api/v1` 调用后端；支付、物流、对象存储和消息队列均通过端口与适配器隔离。

**Tech Stack:** Rust stable、Tokio、Axum、Serde、SQLx、PostgreSQL、Redis、OpenSearch、S3 兼容对象存储、云消息队列、Next.js、React、TypeScript、OpenAPI、Docker、OpenTelemetry。

## Global Constraints

- 后端必须使用 Rust stable、Tokio、Axum、Serde、SQLx；前端使用 Next.js、React、TypeScript。
- 首期使用模块化单体，不因为预期流量直接拆分大量微服务。
- 公开 API 从 `/api/v1` 开始，写操作必须支持 `Idempotency-Key`，请求需要关联 `X-Request-Id`。
- PostgreSQL 是订单、库存、支付状态和结算记录的最终数据源；Redis 和 OpenSearch 数据必须可重建。
- 金额使用货币最小单位整数和 ISO 4217 货币代码，禁止浮点数金额计算。
- 下单前重新校验价格、库存、运费和税费；订单保存商品和金额快照。
- 支付回调、库存扣减、数字交付和异步任务必须幂等。
- 平台不保存银行卡完整信息；支付、退款和卖家结算通过支付服务商适配器完成。
- AI 自动下单需要独立权限 `order:auto_purchase`、服务器侧购买策略和审计记录。
- 代码标识使用英文；注释说明业务原因、状态不变量、安全约束和外部兼容性。
- 每个任务结束时运行该任务的测试和质量检查，再创建一个可审查的 Git 提交。
- 生产环境必须先完成首发区域、支付服务商、卖家结算、税费、隐私、受限商品和消费者保护评估。

---

## 1. 实施范围与阶段拆分

实现拆成五个可以独立验证的交付阶段：

1. **工程基础**：Cargo workspace、Next.js 基础应用、统一错误、请求 ID、配置、数据库迁移和 CI。
2. **商品与交易核心**：身份、卖家、商品、SKU、库存、搜索、购物车、结算预览和订单状态机。
3. **支付与交付**：支付沙盒、Webhook 幂等、事务消息外发、数字交付、实物发货和物流状态。
4. **AI 与治理**：AI 授权、购买策略、AI 搜索和加购、自动下单、审计、举报和管理员处理。
5. **前端与上线准备**：买家与卖家页面、结算页面、订单页面、管理界面、监控、负载测试和安全检查。

首期实现顺序固定为 1 → 2 → 3 → 4 → 5。任何阶段都不得绕过订单、支付、库存和交付领域模块直接写数据库。

## 2. 文件地图

工程基础完成后，核心文件结构如下：

```text
market-bot/
├── apps/
│   ├── api/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── app.rs
│   │       ├── config.rs
│   │       ├── http/
│   │       └── middleware/
│   ├── worker/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── jobs/
│   │       └── consumers/
│   └── web/
│       ├── package.json
│       ├── src/app/
│       ├── src/components/
│       ├── src/features/
│       └── src/lib/
├── crates/
│   ├── shared/
│   ├── identity/
│   ├── seller/
│   ├── catalog/
│   ├── search/
│   ├── cart/
│   ├── order/
│   ├── payment/
│   ├── fulfillment/
│   ├── ai_agent/
│   └── moderation/
├── migrations/
├── tests/
├── openapi/
├── infra/
├── scripts/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
└── .env.example
```

---

## Task 1: 建立 Rust workspace、Web 应用和质量门禁

**Files:**

- Create: `Cargo.toml`
- Create: `Cargo.lock`
- Create: `rust-toolchain.toml`
- Create: `apps/api/Cargo.toml`
- Create: `apps/api/src/lib.rs`
- Create: `apps/api/src/config.rs`
- Create: `apps/api/src/http/mod.rs`
- Create: `apps/api/src/middleware/mod.rs`
- Create: `apps/api/src/main.rs`
- Create: `apps/api/src/app.rs`
- Create: `apps/worker/Cargo.toml`
- Create: `apps/worker/src/main.rs`
- Create: `crates/shared/Cargo.toml`
- Create: `crates/shared/src/lib.rs`
- Create: `tests/Cargo.toml`
- Create: `tests/src/lib.rs`
- Create: `crates/identity/Cargo.toml`
- Create: `crates/identity/src/lib.rs`
- Create: `crates/seller/Cargo.toml`
- Create: `crates/seller/src/lib.rs`
- Create: `crates/catalog/Cargo.toml`
- Create: `crates/catalog/src/lib.rs`
- Create: `crates/search/Cargo.toml`
- Create: `crates/search/src/lib.rs`
- Create: `crates/cart/Cargo.toml`
- Create: `crates/cart/src/lib.rs`
- Create: `crates/order/Cargo.toml`
- Create: `crates/order/src/lib.rs`
- Create: `crates/payment/Cargo.toml`
- Create: `crates/payment/src/lib.rs`
- Create: `crates/fulfillment/Cargo.toml`
- Create: `crates/fulfillment/src/lib.rs`
- Create: `crates/ai_agent/Cargo.toml`
- Create: `crates/ai_agent/src/lib.rs`
- Create: `crates/moderation/Cargo.toml`
- Create: `crates/moderation/src/lib.rs`
- Create: `apps/web/package.json`
- Create: `apps/web/src/app/page.tsx`
- Create: `.github/workflows/ci.yml`
- Create: `.env.example`
- Create: `scripts/check.sh`
- Create: `apps/api/tests/health.rs`

**Interfaces:**

- Produces `market-bot-api`、`market-bot-worker`、`market-bot-shared` 和 `market-bot-tests` Cargo package。
- Produces `build_app() -> axum::Router`，用于 API 测试和运行时启动。
- Produces `GET /healthz`，响应 HTTP 200 和 `{"status":"ok"}`。
- Produces Web 应用首页，显示 `Market Bot` 和当前设计阶段提示。

- [ ] **Step 1: 写健康检查失败测试**

在 `apps/api/tests/health.rs` 中先写测试，测试通过 `build_app()` 发送请求并断言状态码和正文。测试应调用 `tower::ServiceExt::oneshot`，避免依赖真实端口。

```rust
use axum::{body::Body, http::Request};
use market_bot_api::app::build_app;
use tower::ServiceExt;

#[tokio::test]
async fn should_return_ok_from_health_endpoint() {
    let response = build_app()
        .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}
```

- [ ] **Step 2: 运行测试确认失败**

运行：`cargo test -p market-bot-api --test health`

预期：由于 workspace、`build_app` 或 `/healthz` 尚未定义，测试失败。

- [ ] **Step 3: 创建 workspace 和最小 API 应用**

根 `Cargo.toml` 必须声明 `apps/api`、`apps/worker`、`tests` 和全部业务 crate：`shared`、`identity`、`seller`、`catalog`、`search`、`cart`、`order`、`payment`、`fulfillment`、`ai_agent`、`moderation`，并设置 `resolver = "2"`。每个业务 crate 在本任务中创建最小 `Cargo.toml` 和 `src/lib.rs`，使 workspace 从基础阶段即可通过编译；业务实现留给后续任务。`tests/Cargo.toml` 创建 `market-bot-tests` 测试包，并用显式 `[[test]]` 路径承载根 `tests/` 下的跨模块测试。`apps/api/src/lib.rs` 导出 `pub mod app;`，使集成测试可以引用 `market_bot_api::app::build_app`。`apps/api/src/app.rs` 创建包含 `/healthz` 的 Router，`apps/api/src/main.rs` 绑定配置的监听地址。

`apps/api/Cargo.toml` 的包名必须为 `market-bot-api`，并启用 `lib` 目标；Worker 和各业务 crate 也要使用与目录一致的明确包名。
`build_app` 的最小接口如下：

```rust
pub fn build_app() -> axum::Router {
    axum::Router::new().route("/healthz", axum::routing::get(|| async { "{\"status\":\"ok\"}" }))
}
```

```toml
# tests/Cargo.toml
[package]
name = "market-bot-tests"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
```

`tests/src/lib.rs` 可以保持为空的测试包库入口。每新增一个跨模块测试文件时，在同一任务中向 `tests/Cargo.toml` 追加对应的显式 `[[test]]` 目标和所需依赖，避免测试源文件尚未创建时导致 workspace 无法构建。

- [ ] **Step 4: 创建 Worker、Web 和检查脚本**

Worker 先启动 Tokio runtime 并输出结构化启动日志，不消费真实任务。Web 应用先提供可访问首页。`scripts/check.sh` 依次执行格式检查、Clippy、workspace 测试和 Web 类型检查。

- [ ] **Step 5: 运行基础质量检查**

运行：

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
npm --prefix apps/web install
npm --prefix apps/web run typecheck
```

预期：所有命令成功；`GET /healthz` 测试通过。

- [ ] **Step 6: 提交工程基础**

```text
git add Cargo.toml Cargo.lock rust-toolchain.toml apps crates tests .github scripts .env.example
git commit -m "build: bootstrap Rust workspace and web app"
```

---

## Task 2: 实现共享领域内核、错误、金额和请求上下文

**Files:**

- Create: `crates/shared/src/money.rs`
- Create: `crates/shared/src/ids.rs`
- Create: `crates/shared/src/pagination.rs`
- Create: `crates/shared/src/errors.rs`
- Create: `crates/shared/src/request_context.rs`
- Modify: `crates/shared/src/lib.rs`
- Create: `apps/api/src/middleware/request_id.rs`
- Create: `crates/shared/src/money_tests.rs`
- Create: `crates/shared/src/error_tests.rs`

**Interfaces:**

- `Money { minor: i64, currency: CurrencyCode }`
- `Money::new(minor: i64, currency: CurrencyCode) -> Result<Money, MoneyError>`
- `Money::checked_add(self, other: Money) -> Result<Money, MoneyError>`
- `RequestContext { request_id: Uuid, idempotency_key: Option<String> }`
- `ApiError { code: ErrorCode, message: String, request_id: Uuid }`
- `Page<T> { items: Vec<T>, next_cursor: Option<String> }`

- [ ] **Step 1: 为金额不变量编写失败测试**

测试必须覆盖：同币种加法成功、不同币种加法失败、负金额拒绝、货币代码格式校验和整数不溢出。

```rust
#[test]
fn should_reject_addition_for_different_currencies() {
    let usd = Money::new(100, CurrencyCode::try_from("USD").unwrap()).unwrap();
    let eur = Money::new(100, CurrencyCode::try_from("EUR").unwrap()).unwrap();

    assert!(usd.checked_add(eur).is_err());
}
```

- [ ] **Step 2: 运行共享内核测试确认失败**

运行：`cargo test -p market-bot-shared money`

预期：类型或方法未定义导致失败。

- [ ] **Step 3: 实现金额、货币和 ID 类型**

金额类型只保存最小单位整数和三位大写货币代码；所有加减法使用 checked 运算。订单和业务实体 ID 使用 UUID 新类型包装，避免不同实体的 ID 混用。

- [ ] **Step 4: 实现统一错误和请求上下文**

为输入错误、认证错误、权限错误、业务状态错误、外部依赖错误和系统错误定义稳定错误码。错误响应统一序列化 `code`、`message`、`request_id`。中间件优先读取传入的 `X-Request-Id`，格式无效时生成新的 UUID。

- [ ] **Step 5: 运行通过检查并提交**

运行：`cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test -p market-bot-shared`

提交：`git add crates/shared apps/api/src/middleware && git commit -m "feat(shared): add domain primitives and request context"`

---

## Task 3: 建立身份、卖家、商品、SKU 和库存模型

**Files:**

- Create: `crates/identity/src/domain/user.rs`
- Create: `crates/identity/src/application/register_user.rs`
- Create: `crates/seller/src/domain/seller_profile.rs`
- Create: `crates/seller/src/domain/store.rs`
- Create: `crates/catalog/src/domain/product.rs`
- Create: `crates/catalog/src/domain/product_variant.rs`
- Create: `crates/catalog/src/domain/inventory.rs`
- Create: `crates/catalog/src/application/create_product.rs`
- Create: `crates/catalog/src/application/reserve_stock.rs`
- Create: `crates/catalog/src/ports/catalog_repository.rs`
- Create: `migrations/20260902140000_create_identity_catalog.sql`
- Modify: `tests/Cargo.toml`
- Create: `tests/catalog_inventory.rs`
- Create: `apps/api/src/http/routes.rs`

**Interfaces:**

- `CreateProductCommand { seller_id, title, description, fulfillment_type, price_minor, currency }`
- `CatalogService::create_product(command) -> Result<Product, CatalogError>`
- `InventoryService::reserve(variant_id, quantity, reservation_id) -> Result<StockReservation, InventoryError>`
- `InventoryService::release(reservation_id) -> Result<(), InventoryError>`
- `ProductType::{Digital, PhysicalStandard}`

- [ ] **Step 1: 编写商品发布和并发库存失败测试**

`tests/catalog_inventory.rs` 必须覆盖：卖家可以创建商品、缺少价格或交付类型的商品被拒绝、库存不足不能预留、同一个幂等预留请求重复执行不会扣两次库存。

- [ ] **Step 2: 创建数据库迁移**

迁移创建用户、卖家资料、店铺、商品、商品变体和库存表。金额列使用 `BIGINT`，货币列使用 `CHAR(3)`，时间列使用带时区的时间类型；外键、唯一约束和状态约束必须在数据库中声明。

- [ ] **Step 3: 实现领域实体和值对象**

`Product` 只允许 `Draft`、`PendingReview`、`Published`、`Suspended` 和 `Archived` 的合法转换。`ProductVariant` 保存 SKU 和库存策略。库存预留使用预留 ID 幂等，释放只能作用于未释放的预留。

- [ ] **Step 4: 实现 SQLx repository 和用例**

repository 只返回领域对象或明确的持久化错误。库存预留在一个 PostgreSQL 事务中完成，使用条件更新保证 `available_stock >= requested_quantity`。

- [ ] **Step 5: 暴露卖家和商品 API**

实现：

```text
POST /api/v1/seller/stores
POST /api/v1/seller/products
GET  /api/v1/products/{product_id}
PATCH /api/v1/seller/products/{product_id}
```

所有写接口执行用户权限、商品类型、价格、货币、库存和必要字段检查。

- [ ] **Step 6: 运行集成测试并提交**

运行：`cargo test -p market-bot-tests --test catalog_inventory`

提交：`git add crates/identity crates/seller crates/catalog migrations tests apps/api && git commit -m "feat(catalog): add sellers products variants and inventory"`

---

## Task 4: 建立搜索索引和结算前商品确认

**Files:**

- Create: `crates/search/src/index_document.rs`
- Create: `crates/search/src/search_products.rs`
- Create: `crates/search/src/ports/search_repository.rs`
- Create: `apps/worker/src/jobs/index_product.rs`
- Create: `apps/api/src/http/handlers/mod.rs`
- Create: `apps/api/src/http/handlers/product_search.rs`
- Create: `openapi/market-bot.yaml`
- Create: `tests/search_contract.rs`

**Interfaces:**

- `ProductSearchDocument { product_id, variant_ids, title, category_ids, attributes, price_minor, currency, available_stock, fulfillment_type }`
- `SearchProductsQuery { query, category_id, currency, min_price_minor, max_price_minor, cursor }`
- `SearchService::search(query) -> Result<Page<ProductSearchResult>, SearchError>`
- `IndexProductJob { product_id, version }`

- [ ] **Step 1: 编写搜索契约失败测试**

测试 `/api/v1/products/search` 返回统一分页结构、只返回已发布且可售商品、响应字段符合 OpenAPI，并且搜索结果中的价格不能作为最终下单价格。

- [ ] **Step 2: 定义 OpenAPI 资源和错误响应**

在 `openapi/market-bot.yaml` 中定义商品搜索、商品详情、分页、货币、价格和 `PRODUCT_OUT_OF_STOCK` 等错误码。公共 API JSON 字段统一使用 `snake_case`。

- [ ] **Step 3: 实现索引文档和 OpenSearch 适配器**

索引只保存检索需要的商品字段和当前可见库存提示。商品下架、卖家被限制或索引失败时，搜索结果必须过滤或标记不可售；订单模块不能读取索引作为事实来源。

- [ ] **Step 4: 实现索引 Worker 和搜索 handler**

商品发布、价格变化和库存变化写入 outbox 后由 Worker 建立或更新索引。handler 支持关键词、分类、价格、币种和游标分页，并生成 `X-Request-Id` 关联日志。

- [ ] **Step 5: 运行契约和集成测试并提交**

运行：`cargo test --test search_contract`

提交：`git add crates/search apps/worker apps/api/src/http openapi tests && git commit -m "feat(search): add product indexing and search contract"`

---

## Task 5: 实现购物车、结算预览、订单状态机和库存快照

**Files:**

- Create: `crates/cart/src/domain/cart.rs`
- Create: `crates/cart/src/application/update_cart.rs`
- Create: `crates/order/src/domain/order.rs`
- Create: `crates/order/src/domain/order_status.rs`
- Create: `crates/order/src/application/preview_checkout.rs`
- Create: `crates/order/src/application/create_order.rs`
- Create: `crates/order/src/ports/order_repository.rs`
- Create: `migrations/20260902160000_create_cart_order.sql`
- Create: `apps/api/src/http/handlers/cart.rs`
- Create: `apps/api/src/http/handlers/order.rs`
- Create: `tests/order_state_machine.rs`
- Create: `tests/order_concurrency.rs`

**Interfaces:**

- `CartItemSource::{User, Ai}`
- `CheckoutPreview { items, subtotal, shipping_fee, tax, total, expires_at }`
- `CreateOrderCommand { buyer_id, cart_id, shipping_address, payment_method_reference, idempotency_key }`
- `OrderService::preview_checkout(cart_id, buyer_id) -> Result<CheckoutPreview, OrderError>`
- `OrderService::create_order(command) -> Result<Order, OrderError>`
- `Order::transition_to(next_status) -> Result<(), OrderError>`

- [ ] **Step 1: 编写状态机和重复下单失败测试**

覆盖：草稿到待支付可转换、已支付不能回到待支付、重复幂等键返回同一个订单、价格变化返回 `PRICE_CHANGED`、库存不足返回 `PRODUCT_OUT_OF_STOCK`、两个并发请求最多成功一个库存预留。

- [ ] **Step 2: 创建购物车和订单迁移**

购物车表保存所有者和过期时间；购物车明细保存来源。订单表保存状态、买家、结算金额、货币、请求 ID 和幂等键；订单明细保存商品、SKU、卖家、价格、运费、税费和交付规则快照。

- [ ] **Step 3: 实现购物车用例**

购物车添加商品时只保存待确认内容。商品被删除、下架或库存不足时，购物车可以读取但结算预览必须返回明确问题。AI 加购和用户加购共用库存校验，但保留不同来源。

- [ ] **Step 4: 实现结算预览**

结算预览从 PostgreSQL 读取商品和库存，计算价格、运费和税费，并返回有效期。搜索索引中的价格只用于展示，不能用于结算事实。

- [ ] **Step 5: 实现订单创建事务**

一个事务完成：检查幂等键、重新读取商品和价格、预留库存、创建订单、创建订单明细快照、写入支付准备 outbox 事件。支付模块在 Task 6 消费该事件并创建支付记录，避免订单模块直接依赖第三方支付实现。重复请求返回第一次创建的订单，不重复预留库存。

- [ ] **Step 6: 暴露购物车和订单 API**

实现：

```text
GET    /api/v1/carts/{cart_id}
POST   /api/v1/carts/{cart_id}/items
PATCH  /api/v1/carts/{cart_id}/items/{item_id}
DELETE /api/v1/carts/{cart_id}/items/{item_id}
POST   /api/v1/checkout/preview
POST   /api/v1/orders
GET    /api/v1/orders/{order_id}
POST   /api/v1/orders/{order_id}/cancel
```

- [ ] **Step 7: 运行并发和状态机测试并提交**

运行：`cargo test --test order_state_machine --test order_concurrency`

提交：`git add crates/cart crates/order migrations apps/api/src/http tests && git commit -m "feat(order): add cart checkout and order state machine"`

---

## Task 6: 接入支付沙盒、Webhook 幂等和事务消息外发

**Files:**

- Create: `crates/payment/src/domain/payment.rs`
- Create: `crates/payment/src/domain/refund.rs`
- Create: `crates/payment/src/ports/payment_provider.rs`
- Create: `crates/payment/src/adapters/sandbox_provider.rs`
- Create: `crates/payment/src/application/process_webhook.rs`
- Create: `crates/payment/src/application/request_refund.rs`
- Create: `crates/shared/src/outbox.rs`
- Create: `apps/worker/src/jobs/publish_outbox.rs`
- Create: `apps/api/src/http/handlers/payment_webhook.rs`
- Create: `migrations/20260902180000_create_payment_outbox.sql`
- Create: `tests/payment_webhook_idempotency.rs`

**Interfaces:**

- `PaymentProvider::create_payment_intent(input) -> Result<PaymentIntent, ProviderError>`
- `PaymentProvider::verify_webhook(headers, body) -> Result<VerifiedPaymentEvent, ProviderError>`
- `PaymentEventHandler::handle(event) -> Result<PaymentHandlingResult, PaymentError>`
- `RefundService::request_refund(input) -> Result<Refund, PaymentError>`
- `OutboxPublisher::publish_pending(batch_size) -> Result<usize, OutboxError>`

- [ ] **Step 1: 编写重复回调和乱序事件测试**

测试必须验证：同一事件 ID 第二次处理不改变状态；无效签名被拒绝；支付成功事件只触发一次交付事件；退款事件不能覆盖更晚的支付事实；暂时不可用的外部服务会保留可重试任务。

- [ ] **Step 2: 定义支付端口和沙盒适配器**

支付领域只依赖 `PaymentProvider`。沙盒适配器返回确定性的支付意图和测试事件，生产适配器不进入核心领域模块。平台保存支付服务商引用，不保存银行卡完整信息。

- [ ] **Step 3: 实现 Webhook 验证和幂等处理**

验证签名、事件时间窗口和事件 ID。处理事务中检查事件是否已消费，再按照支付状态机更新支付和订单状态，并写入交付 outbox 事件。

- [ ] **Step 4: 实现 Outbox 发布和重试**

业务事务同时写业务变化和 outbox 记录。Publisher 使用租约读取未发布事件，成功后标记已发布；失败增加重试次数和下一次执行时间，超过阈值写入死信记录。

- [ ] **Step 5: 实现退款申请**

退款申请必须检查订单状态、售后期限、已退款金额和争议状态，生成退款记录并通过支付端口执行。退款回调也使用事件 ID 幂等处理。

- [ ] **Step 6: 运行支付集成测试并提交**

运行：`cargo test --test payment_webhook_idempotency`

提交：`git add crates/payment crates/shared/src/outbox.rs apps/worker apps/api/src/http migrations tests && git commit -m "feat(payment): add sandbox payments and idempotent webhooks"`

---

## Task 7: 实现数字交付、实物发货和卖家结算状态

**Files:**

- Create: `crates/fulfillment/src/domain/digital_delivery.rs`
- Create: `crates/fulfillment/src/domain/shipment.rs`
- Create: `crates/fulfillment/src/ports/object_storage.rs`
- Create: `crates/fulfillment/src/ports/logistics_provider.rs`
- Create: `crates/fulfillment/src/adapters/s3_storage.rs`
- Create: `crates/fulfillment/src/adapters/sandbox_logistics.rs`
- Create: `crates/fulfillment/src/application/fulfill_digital_order.rs`
- Create: `crates/fulfillment/src/application/update_shipment.rs`
- Create: `crates/payment/src/application/release_settlement.rs`
- Create: `apps/worker/src/jobs/fulfill_digital_order.rs`
- Create: `apps/worker/src/jobs/sync_shipment_status.rs`
- Create: `migrations/20260902200000_create_fulfillment_settlement.sql`
- Create: `tests/digital_fulfillment.rs`
- Create: `tests/shipment_state.rs`

**Interfaces:**

- `DigitalDeliveryService::fulfill(order_id) -> Result<DeliveryReceipt, FulfillmentError>`
- `ObjectStorage::create_download_url(asset_id, expires_at) -> Result<DownloadUrl, StorageError>`
- `LogisticsProvider::get_tracking_status(tracking_number) -> Result<ShipmentStatus, LogisticsError>`
- `SettlementService::mark_eligible(order_id) -> Result<Settlement, SettlementError>`

- [ ] **Step 1: 编写一次性数字交付和物流状态测试**

覆盖：支付成功订单只生成一次交付凭证、下载链接有过期时间、一次性卡密不会重复分配、实物物流状态可以从已发货更新到已送达、重复物流回调不会倒退状态。

- [ ] **Step 2: 创建数字资产、交付、物流和结算迁移**

数字资产保存加密引用、资产类型和分配状态；交付记录保存订单、凭证引用、下载次数和时间；物流记录保存单号、承运商和统一状态；结算记录保存卖家、金额、资格时间和外部结算引用。

- [ ] **Step 3: 实现对象存储和数字交付适配器**

商品文件使用对象存储私有桶。Worker 生成限时下载地址，不把原始存储路径返回客户端。卡密和兑换码加密保存，分配时在事务内从未分配集合中锁定一条记录。

- [ ] **Step 4: 实现实物发货和物流同步**

卖家提交物流单号后，订单进入 `Shipped`。物流适配器将外部状态映射到 `LabelCreated`、`InTransit`、`Delivered`、`Exception` 和 `Returned`。状态只能按规则前进，异常状态进入人工处理队列。

- [ ] **Step 5: 实现卖家结算资格**

数字商品成功交付、实物商品确认收货或达到自动确认期限后，订单进入可结算状态。退款、争议和物流异常期间不能释放结算。实际资金转移由支付服务商适配器执行。

- [ ] **Step 6: 运行交付测试并提交**

运行：`cargo test --test digital_fulfillment --test shipment_state`

提交：`git add crates/fulfillment crates/payment/src/application/release_settlement.rs apps/worker migrations tests && git commit -m "feat(fulfillment): add digital delivery and shipment tracking"`

---

## Task 8: 实现 AI 授权、购物策略和自动下单

**Files:**

- Create: `crates/ai_agent/src/domain/authorization.rs`
- Create: `crates/ai_agent/src/domain/purchase_policy.rs`
- Create: `crates/ai_agent/src/domain/ai_action.rs`
- Create: `crates/ai_agent/src/application/search_products.rs`
- Create: `crates/ai_agent/src/application/add_to_cart.rs`
- Create: `crates/ai_agent/src/application/auto_purchase.rs`
- Create: `crates/ai_agent/src/ports/authorization_repository.rs`
- Create: `apps/api/src/http/handlers/ai.rs`
- Create: `migrations/20260902210000_create_ai_authorization_audit.sql`
- Create: `tests/ai_authorization.rs`
- Create: `tests/ai_auto_purchase_policy.rs`

**Interfaces:**

- `AiScope::{CatalogRead, CartRead, CartWrite, CheckoutPreview, OrderCreate, OrderRead, AutoPurchase}`
- `PurchasePolicy { allowed_categories, max_order_minor, max_daily_minor, max_monthly_minor, max_shipping_minor, allowed_seller_score, require_price_reconfirmation }`
- `AiAuthorizationService::authorize(subject, client, scopes, expires_at) -> Result<Authorization, AiError>`
- `AiAgentService::add_to_cart(input) -> Result<CartItem, AiError>`
- `AiAgentService::auto_purchase(input) -> Result<Order, AiError>`

- [ ] **Step 1: 编写 AI 越权和预算失败测试**

测试必须验证：没有 `cart:write` 不能加购；没有 `order:auto_purchase` 不能自动购买；自动购买未开启时不能付款；单笔、每日或每月超限时被拒绝；价格或运费超出策略时返回需要用户确认；每个动作都有审计记录。

- [ ] **Step 2: 创建授权、策略和审计迁移**

授权表保存用户、AI 客户端、权限范围、有效期和撤销时间；策略表保存金额最小单位、货币、商品分类、卖家条件、运费和价格确认规则；动作表保存输入摘要、结果、请求 ID、订单 ID 和错误码。

- [ ] **Step 3: 实现授权和策略领域对象**

权限校验在服务器端执行。策略匹配需要明确返回 `Allowed`、`RequiresUserConfirmation` 或 `Blocked`，不能让 AI 自己决定结果。策略计算使用订单最终金额快照，不使用搜索结果金额。

- [ ] **Step 4: 实现 AI 搜索和加购 API**

实现：

```text
GET  /api/v1/ai/products/search
POST /api/v1/ai/carts/{cart_id}/items
GET  /api/v1/ai/orders/{order_id}
```

AI 加购请求保存来源 `Ai`，响应包含商品选择所需的价格、卖家、库存提示、运费、税费和交付信息。

- [ ] **Step 5: 实现自动下单 API**

实现 `POST /api/v1/ai/orders`。服务端按顺序执行授权检查、策略检查、商品重新读取、库存和价格确认、订单创建、支付授权和审计写入。支付凭证只作为支付服务商引用传递，不暴露给 AI。

- [ ] **Step 6: 运行 AI 安全测试并提交**

运行：`cargo test --test ai_authorization --test ai_auto_purchase_policy`

提交：`git add crates/ai_agent apps/api/src/http/handlers/ai.rs migrations tests && git commit -m "feat(ai): add scoped search cart and auto purchase"`

---

## Task 9: 实现商品检查、举报和管理员处理

**Files:**

- Create: `crates/moderation/src/domain/moderation_case.rs`
- Create: `crates/moderation/src/application/review_product.rs`
- Create: `crates/moderation/src/application/create_report.rs`
- Create: `crates/moderation/src/ports/content_scanner.rs`
- Create: `crates/moderation/src/adapters/sandbox_scanner.rs`
- Create: `apps/api/src/http/handlers/moderation.rs`
- Create: `migrations/20260902220000_create_moderation.sql`
- Create: `tests/moderation_visibility.rs`

**Interfaces:**

- `ModerationDecision::{Approved, Rejected, Suspended, NeedsReview}`
- `ContentScanner::scan(asset) -> Result<ScanResult, ScannerError>`
- `ModerationService::review_product(product_id, decision, reason) -> Result<(), ModerationError>`
- `ModerationService::create_report(input) -> Result<ModerationCase, ModerationError>`

- [ ] **Step 1: 编写商品可见性和举报测试**

测试必须验证：未通过基础检查的商品不出现在公开搜索；被暂停商品不能加入购物车；管理员处理记录包含操作者、原因和时间；数字文件扫描失败不会触发交付。

- [ ] **Step 2: 创建审核和举报迁移**

保存商品审核结果、文件扫描结果、举报、处理状态、处理人和原因。审计记录不能包含原始卡密、完整支付令牌或不必要的个人地址。

- [ ] **Step 3: 实现基础检查和状态联动**

商品公开前检查必填字段、商品类型、价格、库存、文件安全结果和卖家状态。审核决定通过事件写入 outbox，触发搜索索引更新；暂停决定触发搜索隐藏。

- [ ] **Step 4: 暴露管理员 API 并测试**

实现商品审核、商品暂停、举报创建和举报处理接口。管理员 API 必须单独认证、授权和审计。

- [ ] **Step 5: 运行测试并提交**

运行：`cargo test --test moderation_visibility`

提交：`git add crates/moderation apps/api/src/http/handlers/moderation.rs migrations tests && git commit -m "feat(moderation): add product review and reports"`

---

## Task 10: 实现 Next.js 买家、卖家、购物车和订单界面

**Files:**

- Create: `apps/web/src/lib/api-client.ts`
- Create: `apps/web/src/lib/types.ts`
- Create: `apps/web/src/features/catalog/ProductSearch.tsx`
- Create: `apps/web/src/features/catalog/ProductDetail.tsx`
- Create: `apps/web/src/features/cart/CartPanel.tsx`
- Create: `apps/web/src/features/checkout/CheckoutPreview.tsx`
- Create: `apps/web/src/features/orders/OrderStatus.tsx`
- Create: `apps/web/src/features/seller/ProductEditor.tsx`
- Create: `apps/web/src/app/products/page.tsx`
- Create: `apps/web/src/app/products/[productId]/page.tsx`
- Create: `apps/web/src/app/cart/page.tsx`
- Create: `apps/web/src/app/checkout/page.tsx`
- Create: `apps/web/src/app/orders/[orderId]/page.tsx`
- Create: `apps/web/src/app/seller/products/new/page.tsx`
- Create: `apps/web/src/features/checkout/CheckoutPreview.test.tsx`

**Interfaces:**

- `ApiClient.get<T>(path, options) -> Promise<T>`
- `ApiClient.post<T>(path, body, options) -> Promise<T>`
- `ProductCard` 接收结构化商品、价格、货币、库存和交付类型。
- `CheckoutPreview` 必须显示商品快照价格、运费、税费、总额、有效期和价格变化提示。

- [ ] **Step 1: 编写结算显示失败测试**

测试结算组件正确显示多币种金额、运费和税费；商品价格变化时显示需要重新确认；数字商品显示交付方式，实物商品显示配送信息。

- [ ] **Step 2: 创建类型化 API 客户端**

API 客户端统一处理认证、`X-Request-Id`、`Idempotency-Key`、错误码和 JSON `snake_case` 字段。组件不能直接拼接订单或支付状态字符串。

- [ ] **Step 3: 实现商品搜索和详情页面**

页面提供关键词、分类、价格、货币和交付类型筛选。详情页显示卖家、库存提示、交付规则、退款规则和加入购物车按钮。

- [ ] **Step 4: 实现购物车和结算页面**

购物车区分用户加购和 AI 加购来源，但两者使用相同结算校验。结算页在提交前显示最终价格、库存、运费、税费和支付服务商跳转状态。

- [ ] **Step 5: 实现订单和卖家发布页面**

订单页显示订单、支付、交付和物流独立状态。卖家表单根据数字或实物类型显示对应字段，提交前展示文件安全检查、库存和交付规则。

- [ ] **Step 6: 运行 Web 检查并提交**

运行：

```text
npm --prefix apps/web run lint
npm --prefix apps/web run typecheck
npm --prefix apps/web test -- --runInBand
npm --prefix apps/web run build
```

提交：`git add apps/web && git commit -m "feat(web): add buyer seller cart and order flows"`

---

## Task 11: 建立部署、监控、备份和安全门禁

**Files:**

- Create: `infra/docker/api.Dockerfile`
- Create: `infra/docker/worker.Dockerfile`
- Create: `infra/docker/web.Dockerfile`
- Create: `infra/compose/docker-compose.dev.yml`
- Create: `infra/terraform/modules/app/main.tf`
- Create: `infra/terraform/modules/data/main.tf`
- Create: `infra/terraform/environments/staging/main.tf`
- Create: `apps/api/src/telemetry.rs`
- Create: `apps/worker/src/telemetry.rs`
- Create: `scripts/load_test.sh`
- Create: `scripts/security_check.sh`
- Create: `docs/operations/runbook.md`
- Modify: `.github/workflows/ci.yml`
- Modify: `tests/Cargo.toml`
- Create: `tests/health_smoke.rs`

**Interfaces:**

- API、Worker 和 Web 都使用非 root 运行时镜像。
- API 和 Worker 从环境变量或密钥管理服务读取配置。
- OpenTelemetry span 使用 `request_id`、`order_id`、`payment_id` 和 `event_id` 关联。
- `scripts/load_test.sh` 输出搜索、结算预览、订单查询的延迟和错误率。

- [ ] **Step 1: 编写健康、配置和敏感日志测试**

测试缺少必须配置时应用拒绝启动；日志过滤密码、卡密、支付令牌和完整地址；健康检查不暴露数据库连接字符串或密钥。

- [ ] **Step 2: 创建本地开发基础设施**

Docker Compose 启动 PostgreSQL、Redis、OpenSearch、对象存储模拟器和消息队列模拟器。开发环境使用沙盒支付和模拟物流，不使用生产凭证。

- [ ] **Step 3: 创建容器和预发布部署配置**

API、Worker 和 Web 使用独立镜像。数据库、Redis、OpenSearch、对象存储和队列使用托管资源接口；业务代码通过环境变量获取连接信息。生产配置不把密钥写入镜像或 Git。

- [ ] **Step 4: 添加遥测和业务指标**

记录 HTTP 延迟、数据库连接池、队列积压、支付回调失败、数字交付成功率、物流同步失败、AI 规则拦截和死信数量。敏感字段在 logger 层统一过滤。

- [ ] **Step 5: 添加负载和安全检查**

负载测试覆盖商品搜索、商品详情、购物车、结算预览、订单查询和 AI 搜索。安全检查覆盖依赖漏洞、镜像漏洞、密钥扫描、API 限流、令牌撤销、Webhook 签名和文件上传限制。

- [ ] **Step 6: 执行完整质量门禁**

运行：

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
npm --prefix apps/web run lint
npm --prefix apps/web run typecheck
npm --prefix apps/web run build
bash scripts/security_check.sh
bash scripts/load_test.sh
```

- [ ] **Step 7: 提交上线准备**

提交：`git add infra apps scripts docs/operations .github && git commit -m "ops: add deployment observability and release gates"`

---

## 3. 验收清单

以下条件全部满足后才进入受控生产：

- 用户可以注册、完成基础验证并创建店铺。
- 卖家可以发布数字商品和标准实物商品。
- 商品审核、搜索和下架联动正常。
- 买家可以搜索商品、加入购物车、查看结算预览和完成支付沙盒流程。
- 价格、库存、运费和税费在结算前重新确认。
- 重复请求不会创建重复订单或重复扣库存。
- 重复支付回调不会重复更新支付状态或发货。
- 数字商品支付后只交付一次，下载链接受时效和权限限制。
- 实物商品可以提交物流单号并同步物流状态。
- 退款、取消、争议和卖家结算状态互相符合规则。
- AI 可以搜索商品并在授权范围内加购。
- 未开启自动购买、权限不足或超过策略时，AI 不能付款。
- 符合策略的 AI 自动订单会经过服务器重新校验并写入审计日志。
- 管理员可以处理商品审核、举报和风险状态。
- OpenAPI、错误码、请求 ID 和幂等键在客户端和服务端一致。
- 备份、恢复、死信、告警和敏感日志检查通过。
- 首发区域和支付服务商的生产合规条件已经完成评估。

## 4. 计划自检

### 规格覆盖

- 产品角色、商品类型和购买模式：Task 3、Task 5、Task 7、Task 8、Task 9、Task 10。
- Rust 模块化单体和 API 优先：Task 1、Task 2、Task 4、Task 5。
- 商品、库存、购物车和订单：Task 3、Task 5。
- 支付、退款、托管结算和 Webhook：Task 6、Task 7。
- 数字商品和实物交付：Task 7。
- AI 搜索、加购、授权和自动下单：Task 8。
- 开放卖家、审核和风控：Task 3、Task 9。
- 全球化、云托管和可观测性：Task 1、Task 10、Task 11。
- 命名、注释、测试和质量门禁：所有任务，具体规范见 `docs/standards/code-naming-and-comments.md`。

### 占位符检查

计划中的任务都有明确文件、接口、命令、测试目标和提交信息，没有依赖模糊的后续补充。

### 类型和接口一致性

- `Money` 在共享内核定义，商品、订单、支付和结算都使用最小单位金额。
- `build_app()` 在 Task 1 定义，后续 API handler 通过路由注册到同一 Router。
- `PaymentProvider` 在 Task 6 定义，退款和结算通过同一支付端口，不直接依赖第三方 SDK。
- `ProductSearchDocument` 只服务检索；Task 5 的结算预览直接读取 PostgreSQL。
- `AiScope::AutoPurchase` 只由 Task 8 使用，普通 `OrderCreate` 权限不会自动继承。
- `OutboxPublisher` 在 Task 6 定义，商品索引、交付和通知任务使用相同的发布和重试约束。

计划保存后，选择执行方式：

1. **Subagent-Driven**：按任务逐个分派并在每个任务后审查。
2. **Inline Execution**：在当前会话中按阶段执行，并在阶段之间暂停检查。
