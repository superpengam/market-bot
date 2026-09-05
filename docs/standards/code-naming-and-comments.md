# Market Bot 代码命名与注释规范

- 文档版本：0.1.0
- 文档状态：已确认设计基线
- 更新时间：2026-09-02
- 适用范围：Rust 后端、Next.js 前端、数据库、API、异步任务和基础设施配置

## 1. 总体原则

1. 目录和文件按职责分类，禁止把业务、部署、脚本和文档混在一起。
2. 代码标识统一使用英文，保证全球团队和外部 API 客户端可读。
3. 注释说明业务原因、约束和安全风险，不重复描述显而易见的代码行为。
4. 一个模块只负责一个清晰的业务边界。
5. 公开接口、数据库字段、事件名称和错误码必须稳定且有文档。
6. 任何涉及支付、库存、订单、交付或 AI 权限的代码必须可测试、可审计。

## 2. 顶层目录

```text
market-bot/
├── apps/                         # 可独立运行的应用
│   ├── api/                      # HTTP API 服务
│   └── worker/                   # 异步任务服务
├── crates/                       # Rust 业务模块
├── migrations/                   # 数据库迁移文件
├── tests/                        # 跨模块和端到端测试
├── openapi/                      # API 契约和示例
├── infra/                        # Docker、部署和云资源配置
├── scripts/                      # 可重复执行的脚本
├── docs/                         # 产品和技术文档
├── Cargo.toml
├── Cargo.lock
├── README.md
└── .env.example
```

顶层职责：

- `apps/` 只负责启动应用、组装依赖、注册路由和启动 Worker。
- `crates/` 负责领域规则和业务用例。
- `migrations/` 保存数据库结构变化。
- `tests/` 保存需要多个模块或外部依赖的测试。
- `infra/` 只保存部署和资源配置，不保存业务逻辑。
- `scripts/` 保存可重复运行的辅助命令，不隐藏关键业务流程。
- `docs/` 保存产品、架构和开发规范。

## 3. Rust 命名规则

- 文件名和模块名：`snake_case`，例如 `order_service.rs`。
- Rust 类型、结构体、枚举和 trait：`UpperCamelCase`，例如 `OrderService`。
- 函数、方法和局部变量：`snake_case`，例如 `create_order`。
- 常量和静态变量：`SCREAMING_SNAKE_CASE`，例如 `MAX_RETRY_COUNT`。
- 枚举成员：`UpperCamelCase`，例如 `PaymentStatus::Succeeded`。
- Cargo 包名：小写连字符，例如 `ai-agent`。
- Rust 引用 crate 时使用下划线，例如 `ai_agent`。
- 布尔变量使用 `is_`、`has_`、`can_` 或 `should_` 前缀。
- ID 使用完整实体名称，例如 `product_id`、`seller_id`，禁止使用无意义的 `id2`。
- 时间字段使用明确后缀，例如 `created_at`、`expires_at`、`settled_at`。
- 金额字段使用用途和单位，例如 `subtotal_minor`、`shipping_fee_minor`。

示例：

```rust
pub struct ProductVariant {
    pub product_id: ProductId,
    pub is_active: bool,
    pub available_stock: i64,
}

pub const MAX_AUTO_PURCHASE_AMOUNT_MINOR: i64 = 100_00;
```

## 4. Rust 模块结构

重要业务 crate 使用统一内部结构：

```text
crates/order/src/
├── lib.rs
├── domain/                       # 实体、值对象、领域规则
├── application/                  # 用例和业务编排
├── ports/                        # 外部依赖抽象
├── adapters/                     # 数据库、队列和第三方实现
├── errors.rs                     # 模块错误
└── tests/                        # 模块级测试
```

分层约束：

- `domain` 不能依赖 HTTP、数据库连接或第三方支付 SDK。
- `application` 负责组合领域能力和外部端口。
- `ports` 定义模块需要的接口。
- `adapters` 实现具体技术连接。
- API handler 只负责请求转换、认证、输入校验和响应转换。
- Worker 负责任务消费和重试，不复制订单或支付领域规则。
- `shared` 只保存稳定的跨模块基础类型，不能把业务逻辑都放进去。

## 5. 数据库命名

数据库使用小写 `snake_case`：

```text
product_variants
seller_profiles
available_stock
created_at
updated_at
```

规则：

- 表名使用复数，例如 `products`、`orders`。
- 主键使用 `<entity>_id`。
- 外键使用关联实体名称加 `_id`。
- 布尔字段以 `is_`、`has_` 或 `can_` 开头。
- 金额字段以 `_minor` 结尾，并保存对应货币代码。
- 时间统一保存带时区的 UTC 时间，并使用 `_at` 后缀。
- 状态字段使用受约束的枚举值，禁止让模块写入任意字符串。
- 对外展示名称和内部业务标识分开保存。
- 敏感字段必须明确加密或脱敏策略，不能仅靠字段名称掩盖风险。

迁移文件格式：

```text
YYYYMMDDHHMMSS_<action>_<table>.sql
```

例如：

```text
20260902103000_create_products.sql
20260902104500_add_currency_to_products.sql
```

已提交的迁移文件不可直接修改。结构变更必须新增迁移文件，并提供升级和回滚验证。

## 6. API 命名

- API 版本统一使用 `/api/v1`。
- URL 使用小写连字符和复数资源名，例如 `/product-variants`。
- 单个资源使用 `/{resource_id}`。
- JSON 字段使用项目统一约定的格式；首期公共 API 采用 `snake_case`，避免同一字段在不同接口中改名。
- 写操作要求支持 `Idempotency-Key`。
- 请求使用 `X-Request-Id` 或等效标识关联日志。
- 错误响应至少包含 `code`、`message` 和 `request_id`。
- OpenAPI 中必须说明字段类型、是否必填、约束、错误码和示例。
- 不在 URL、日志或错误消息中暴露密码、支付令牌、卡密和完整个人信息。

示例：

```text
GET    /api/v1/products/search
GET    /api/v1/products/{product_id}
POST   /api/v1/carts/{cart_id}/items
POST   /api/v1/checkout/preview
POST   /api/v1/orders
GET    /api/v1/orders/{order_id}
```

## 7. 事件和异步任务命名

事件使用过去时态，表示已经发生的事实：

```text
order.created
payment.succeeded
payment.refund_requested
digital_fulfillment.completed
shipment.status_updated
ai_order.blocked
```

事件负载必须包含：

- `event_id`
- `event_type`
- `occurred_at`
- `aggregate_type`
- `aggregate_id`
- `request_id`
- 业务数据版本

任务名称使用动作加对象，例如：

```text
index_product
fulfill_digital_order
sync_shipment_status
send_order_notification
reconcile_payment_event
```

所有任务必须支持幂等、重试和死信处理。

## 8. 注释规范

### 8.1 注释原则

注释优先解释：

- 为什么必须这样处理。
- 哪个业务不变量不能破坏。
- 哪个安全风险需要防止。
- 哪个第三方系统行为需要兼容。
- 哪个并发或一致性问题决定了实现方式。

不写只重复代码的注释：

```rust
// 不推荐：给 count 加一。
count += 1;
```

推荐说明业务约束：

```rust
// Why: 预留库存只在订单有效期内保留，过期后必须释放，避免库存长期不可售。
reservation.release_if_expired(now)?;
```

### 8.2 注释标记

复杂业务代码可以使用以下标记：

```rust
// Why: 订单必须保存价格快照，避免商品修改后影响历史订单。

// Invariant: 已支付订单不能直接回到待支付状态。

// Safety: 支付服务商可能重复发送回调，处理前必须检查事件 ID。

// Compatibility: 此字段兼容旧版本 API，移除前需要完成客户端迁移。
```

### 8.3 Rust 文档注释

- 公共结构、函数、trait 和模块使用 `///`。
- crate 或模块级说明使用 `//!`。
- 公共 API 文档说明参数、返回值、错误、权限和幂等要求。
- 状态机方法必须说明允许的前置状态和失败条件。
- 支付、库存、交付和 AI 自动下单方法必须说明安全和业务不变量。

### 8.4 TODO 规则

`TODO` 只能用于已经登记的问题，并且必须包含负责人、问题编号或明确完成条件：

```rust
// TODO(MB-142, payments): 在生产支付服务商确认后移除沙盒回退逻辑。
```

禁止使用没有上下文的 `TODO later`、`FIXME` 或空白占位注释。

## 9. 测试命名和位置

- 单元测试函数：`should_<expected_behavior>`。
- 集成测试文件：`tests/<module>_<scenario>.rs`。
- API 测试文件：`tests/api/<resource>_<action>.rs`。
- 测试数据工厂：`fixtures/<domain>_fixture.rs`。
- 测试夹具使用明确名称，不使用共享可变全局状态。

示例：

```rust
#[test]
fn should_reject_order_when_inventory_is_insufficient() {
    // ...
}
```

支付、库存、订单、交付和 AI 权限至少需要测试：

- 正常流程。
- 重复请求。
- 并发请求。
- 状态转换失败。
- 外部服务临时失败。
- 授权过期或越权。
- 敏感数据不会出现在日志。

## 10. 配置和敏感信息

- 环境变量使用 `SCREAMING_SNAKE_CASE`，例如 `DATABASE_URL`。
- `.env.example` 只保存变量名、格式和非敏感示例。
- `.env`、生产密钥、支付令牌和 API Key 不得提交到版本库。
- 配置读取后转换为类型化配置结构，业务代码不能到处读取环境变量。
- 不同环境使用不同数据库、队列、对象存储桶和支付凭证。
- 任何日志都需要经过敏感字段过滤。

## 11. 前端命名

- React 组件文件使用 `PascalCase.tsx`，例如 `ProductCard.tsx`。
- 页面、路由和普通工具文件遵循 Next.js 约定；业务工具文件使用 `camelCase.ts`。
- React 组件、类型和接口使用 `PascalCase`。
- hooks 使用 `use` 加 `PascalCase`，例如 `useCart.ts`。
- 变量、函数和属性使用 `camelCase`。
- 常量使用 `SCREAMING_SNAKE_CASE`。
- 组件中不直接拼接支付或订单状态字符串，使用共享类型和映射函数。

## 12. 质量检查

提交前至少执行：

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
```

前端项目执行对应的格式化、类型检查、Lint、单元测试和构建命令。涉及数据库、OpenAPI、支付、库存、交付或权限的改动必须执行对应集成测试。

## 13. 提交和审查

提交信息使用简短、明确的英文命令式描述，并按功能拆分：

```text
feat(order): add idempotent order creation
fix(payment): ignore duplicate webhook events
docs(standards): define Rust naming rules
```

一个提交只解决一个可说明的问题。提交前需要确认：

- 没有提交敏感信息。
- 没有修改已发布迁移文件。
- 公共 API 和 OpenAPI 已同步。
- 错误码、事件和审计字段已补充。
- 支付、库存或 AI 权限变更有测试。
- 注释说明了必要的业务原因和安全约束。

## 14. 代码审查清单

### 文件和结构

- 文件是否放在正确的职责目录？
- 模块边界是否清晰？
- 是否把业务规则错误地放进 `shared` 或 API handler？

### 命名

- 标识是否使用统一语言和命名格式？
- ID、金额、时间和状态是否表达了完整含义？
- API、数据库和事件名称是否一致？

### 业务安全

- 是否验证授权和 AI 策略？
- 是否重新校验价格、库存、运费和税费？
- 是否支持幂等和重试？
- 是否避免敏感信息进入日志？

### 注释和测试

- 注释是否解释原因而不是重复代码？
- 状态机和并发规则是否有说明？
- 正常、异常、重复和并发流程是否都有测试？