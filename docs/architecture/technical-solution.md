# Market Bot 技术方案

- 文档版本：0.1.0
- 文档状态：已确认设计基线
- 更新时间：2026-09-02
- 适用阶段：首期 MVP 设计与后续规模化扩展
- 关联文档：`docs/product/product-plan.md`、`docs/standards/code-naming-and-comments.md`

## 1. 技术目标

Market Bot 的技术方案需要同时满足以下目标：

1. 支持数字商品和标准实物商品的统一交易流程。
2. 让网页端、移动端和外部 AI 使用同一套 API。
3. 在库存、支付、订单和交付流程中保证幂等和可审计。
4. 支持全球化所需的多语言、多货币、时区、区域和支付扩展。
5. 允许任何用户成为卖家，同时通过基础验证、内容检查和风控降低滥用风险。
6. 在首期保持架构简单，在流量和业务边界明确后再拆分服务。

## 2. 非目标

首期不实现：

- 一开始就拆分大量微服务。
- 自行保存银行卡完整信息或自行运营资金托管系统。
- 一次性覆盖所有国家和所有支付方式。
- 直接开放账号交易、拍卖、订阅和复杂服务预约。
- 让 AI 绕过平台 API、权限、价格校验或风控直接写入数据库。

## 3. 总体架构

采用 Rust 模块化单体和 API 优先设计。

```text
Web / Mobile / External AI
            │
            ▼
      API Gateway / WAF
            │
            ▼
      Rust API Application
  ┌──────────────────────────┐
  │ Identity                 │
  │ Seller                   │
  │ Catalog                  │
  │ Search                   │
  │ Cart                     │
  │ Order                   │
  │ Payment                 │
  │ Fulfillment             │
  │ AI Agent                │
  │ Moderation              │
  └──────────────────────────┘
       │       │        │
       ▼       ▼        ▼
 PostgreSQL  Redis  OpenSearch
       │
       ▼
 Transactional Outbox
       │
       ▼
 Queue → Rust Worker → Payment / Storage / Logistics / Notifications
```

`API` 和 `Worker` 可以独立部署和扩容，但首期仍属于同一个 Cargo workspace 和统一领域模型。

## 4. 技术栈

### 4.1 前端

- Next.js、React、TypeScript。
- 使用服务端渲染或静态渲染改善商品页面性能。
- 使用统一 API 客户端访问后端。
- 支持多语言、时区、货币和区域化展示。
- 后续可以使用 React Native 开发移动端，复用 API 和共享类型。

### 4.2 后端

- Rust stable。
- Tokio 作为异步运行时。
- Axum 作为 HTTP API 框架。
- Serde 负责 JSON 序列化和反序列化。
- SQLx 负责 PostgreSQL 访问和编译期查询检查。
- Tracing 和 OpenTelemetry 负责结构化日志与链路追踪。
- Cargo workspace 管理应用和业务 crate。

### 4.3 基础设施

- PostgreSQL：系统交易数据的最终数据源。
- Redis：缓存、限流、短期分布式锁和任务辅助数据。
- OpenSearch：商品全文搜索、筛选、排序和后续语义检索。
- S3 兼容对象存储：商品图片、数字文件和交付文件。
- CDN：商品图片和受控制的下载内容分发。
- 云消息队列：异步任务和事件传递。
- Docker：应用构建和部署封装。
- 托管容器服务：首期运行 API 和 Worker。
- WAF、密钥管理和备份服务：平台边界和数据保护。

云服务保持适配器边界。AWS 可以作为参考部署环境，但业务代码不能直接依赖某一个云厂商的不可替换实现。

## 5. Cargo workspace 与目录

```text
market-bot/
├── apps/
│   ├── api/                         # 对外 HTTP API 服务
│   └── worker/                      # 异步任务和交付服务
├── crates/
│   ├── identity/                    # 用户、登录和身份
│   ├── seller/                      # 卖家和店铺
│   ├── catalog/                     # 商品、分类和 SKU
│   ├── search/                      # 搜索和索引
│   ├── cart/                        # 购物车
│   ├── order/                       # 订单和状态机
│   ├── payment/                     # 支付、退款和结算
│   ├── fulfillment/                 # 数字交付和实物配送
│   ├── ai_agent/                    # AI 授权、规则和自动下单
│   ├── moderation/                  # 内容检查和风控
│   └── shared/                      # 公共类型、错误和基础设施
├── migrations/                      # PostgreSQL 迁移
├── tests/                           # 跨模块和端到端测试
├── openapi/                         # API 契约和示例
├── infra/                           # Docker、部署和云资源配置
├── scripts/                         # 可重复运行的开发和运维脚本
├── docs/                            # 产品、架构和规范文档
├── Cargo.toml
├── Cargo.lock
└── .env.example
```

每个重要业务 crate 内部使用以下边界：

```text
crates/order/src/
├── lib.rs
├── domain/                          # 实体、值对象和状态规则
├── application/                     # 用例编排
├── ports/                           # 数据库、队列和外部服务抽象
├── adapters/                        # 具体基础设施实现
├── errors.rs                        # 模块错误定义
└── tests/                            # 模块级测试
```

`domain` 不依赖 HTTP、数据库和第三方 SDK。`application` 负责用例编排。`ports` 定义依赖能力。`adapters` 实现外部连接。API 层只负责请求转换、认证、参数校验和响应映射。

## 6. 模块职责

### Identity

负责注册、登录、会话、基础身份验证、用户状态和授权主体。

### Seller

负责卖家身份、店铺、卖家规则、结算资格和卖家风险信息。

### Catalog

负责商品、分类、标签、规格、SKU、商品状态和价格快照来源。

### Search

负责索引构建、全文搜索、筛选、排序和后续语义搜索。搜索结果不是订单最终数据源。

### Cart

负责购物车、商品数量、来源标记和购物车有效性。来源需要区分用户加购和 AI 加购。

### Order

负责订单创建、订单明细快照、状态机和订单生命周期。订单模块不直接处理银行卡信息。

### Payment

负责支付意图、支付回调、退款、结算状态和支付服务商适配器。

### Fulfillment

负责数字内容交付、下载凭证、实物发货信息和物流状态。

### AI Agent

负责 AI 客户端身份、OAuth 或 API Key、权限范围、购买策略、预算校验和 AI 操作审计。

### Moderation

负责商品基础检查、文件安全扫描、举报、风险评分和人工处理队列。

### Shared

只放跨模块稳定且通用的类型，例如请求 ID、分页、货币值对象、通用错误、时间和事件基础结构。禁止把业务规则集中到 `shared`，避免形成新的大杂烩模块。

## 7. 数据模型和一致性

### 7.1 主要实体

用户和卖家：`users`、`seller_profiles`、`stores`。

商品：`products`、`product_variants`、`inventory_items`、`digital_assets`、`shipping_profiles`、`categories`。

交易：`carts`、`cart_items`、`orders`、`order_items`、`payments`、`refunds`、`settlements`。

交付：`fulfillments`、`shipments`、`delivery_attempts`。

AI 和审计：`ai_policies`、`ai_authorizations`、`ai_actions`、`audit_logs`、`idempotency_records`。

### 7.2 数据原则

- PostgreSQL 是订单、库存、支付状态和结算记录的最终数据源。
- Redis 数据丢失后可以重新生成，不保存无法恢复的订单事实。
- OpenSearch 允许最终一致，下单前必须读取 PostgreSQL 重新确认。
- 订单明细保存商品名称、规格、价格、货币、税费、运费、卖家和交付规则快照。
- 货币使用最小单位整数和 ISO 4217 货币代码。
- 重要金额使用明确字段，例如 `subtotal_minor`、`shipping_fee_minor`。
- 库存扣减使用事务和行锁或乐观锁，并且必须有唯一幂等键。
- 订单、支付、交付和结算状态分别保存，不能用一个状态字段代替全部状态。

### 7.3 事务消息外发

涉及订单状态变化的事务同时写入业务表和 outbox 表。后台任务读取 outbox，投递到消息队列，并记录投递状态。

```text
数据库事务
  ├── 更新订单或库存
  └── 写入 outbox 事件
          ↓
Outbox Publisher
          ↓
消息队列
          ↓
Worker 消费、重试或进入死信队列
```

这样可以避免数据库已经提交但异步交付任务没有创建的问题。

## 8. API 设计

公开接口从 `/api/v1` 开始，使用 REST 和 OpenAPI。

核心接口包括：

```text
GET    /api/v1/products/search
GET    /api/v1/products/{product_id}
GET    /api/v1/stores/{store_id}

GET    /api/v1/carts/{cart_id}
POST   /api/v1/carts/{cart_id}/items
PATCH  /api/v1/carts/{cart_id}/items/{item_id}
DELETE /api/v1/carts/{cart_id}/items/{item_id}

POST   /api/v1/checkout/preview
POST   /api/v1/orders
GET    /api/v1/orders/{order_id}
POST   /api/v1/orders/{order_id}/cancel
POST   /api/v1/orders/{order_id}/refund-request

GET    /api/v1/ai/products/search
POST   /api/v1/ai/carts/{cart_id}/items
POST   /api/v1/ai/orders
GET    /api/v1/ai/orders/{order_id}
```

写操作要求 `Idempotency-Key`。所有请求尽量使用 `X-Request-Id` 或等效请求标识关联日志。错误响应包含稳定的 `code`、用户可读的 `message` 和 `request_id`。

公开 API 的 URL 使用小写连字符和复数资源名；Rust 内部标识和数据库字段使用 `snake_case`。API 字段格式在 OpenAPI 中统一定义，不能由不同模块自行决定。

## 9. 身份、权限和 AI 接入

网页端使用用户会话或 OAuth。外部 AI 使用 OAuth 2.1、受限 API Key 或用户授权令牌。每个外部客户端必须能被撤销、限流和审计。

权限范围包括：

```text
catalog:read
cart:read
cart:write
checkout:preview
order:create
order:read
order:auto_purchase
```

`order:auto_purchase` 必须单独授予，不能从普通下单权限自动继承。

AI 自动购买策略由服务器保存和执行，至少包括商品类别、卖家条件、金额限额、价格变化、配送区域和支付授权。AI 提交的商品和金额不直接成为最终事实，服务器必须重新读取商品和库存并执行规则检查。

后续可以增加 MCP（让 AI 调用外部工具的标准协议）适配器。MCP 只负责工具调用适配，不能绕过 Market Bot 的权限、订单状态机和支付流程。

## 10. 支付与结算

支付模块通过适配器连接支持平台分账、退款和延迟结算的合规支付服务商。Stripe Connect、Adyen MarketPay 等可以作为候选服务，但生产服务商必须根据首发国家、卖家所在地、买家所在地和商品类型评估后确定。

支付流程：

```text
创建待支付订单
  ↓
创建支付意图
  ↓
支付服务商处理付款
  ↓
验证签名回调
  ↓
使用事件 ID 做幂等处理
  ↓
更新支付状态和订单状态
  ↓
触发数字交付或等待卖家发货
```

平台不保存银行卡完整信息。支付回调可能重复、延迟或乱序，因此必须使用状态机和事件唯一标识处理。

## 11. 交付设计

### 11.1 数字交付

- 数字文件加密保存。
- 文件使用限时下载地址。
- 卡密或兑换码加密保存，并在交付时原子标记为已分配。
- 记录订单、交付凭证、下载次数和下载时间。
- Worker 失败时可以安全重试，不能重复发放同一份一次性凭证。

### 11.2 实物配送

- 订单保存收货地址快照和配送选项快照。
- 卖家提交物流单号后创建 shipment。
- 物流适配器把外部状态映射到平台统一状态。
- 物流同步失败进入重试队列，超过重试次数后通知管理员。
- 订单完成和卖家结算依赖统一的收货、自动确认或争议规则。

## 12. 状态机

订单状态至少包括：

```text
Draft
PendingConfirmation
PendingPayment
PaymentProcessing
Paid
FulfillmentProcessing
Shipped
Delivered
Completed
CancellationRequested
Cancelled
RefundProcessing
Refunded
DisputeProcessing
```

支付、交付和结算拥有独立状态机。状态转换由领域模块提供方法，API 层不能直接修改状态字符串。

每次转换记录：

- 前置状态和目标状态。
- 操作主体和主体类型。
- 请求 ID 和幂等键。
- 发生时间。
- 业务原因。
- 关联外部事件 ID。

## 13. 错误处理、重试和恢复

错误分为输入错误、权限错误、业务状态错误、外部依赖错误和系统错误。

推荐的稳定错误码包括：

```text
PRODUCT_OUT_OF_STOCK
PRICE_CHANGED
PAYMENT_REQUIRES_ACTION
AI_AUTHORIZATION_EXPIRED
AUTO_PURCHASE_LIMIT_EXCEEDED
FULFILLMENT_FAILED
ORDER_STATE_INVALID
IDEMPOTENCY_KEY_REUSED
```

处理要求：

- 客户端错误直接返回明确原因，不进行无意义重试。
- 支付、物流、消息队列和文件服务的临时错误使用指数退避重试。
- 不可重试或超过次数的任务进入死信队列。
- 任务处理必须可重复执行，不产生重复扣款、扣库存或发货。
- 数据库事务失败时不发送业务成功事件。
- 监控系统需要区分业务失败、依赖失败和代码异常。

## 14. 安全与隐私

- 密码使用强哈希算法保存。
- 访问令牌、API Key 和支付令牌使用密钥管理系统保存或加密保存。
- 日志禁止包含密码、卡密、完整支付凭证、完整令牌和不必要的个人地址。
- 商品文件上传执行扩展名、MIME 类型、大小和病毒检查。
- 商品文件和交付链接使用最小权限和短时有效期。
- 所有 API、Webhook 和外部服务通信使用 HTTPS。
- 登录、搜索、加购、下单、支付回调和文件下载设置限流。
- 管理操作、支付操作、库存操作和 AI 操作写入审计日志。
- PII（个人可识别信息）按照最小权限访问、区域规则和保留期限处理。
- 对外 API 实施请求签名、来源校验、权限校验和重放防护。

## 15. 部署与扩容

首期采用单区域、托管服务和容器部署：

```text
CDN / WAF
   ↓
Load Balancer
   ↓
API Containers ───── Worker Containers
   │                         │
Managed PostgreSQL      Message Queue
Redis                   Object Storage
OpenSearch              Payment / Logistics APIs
```

部署要求：

- API 和 Worker 无状态，支持横向扩容。
- 数据库启用自动备份、恢复演练和必要的读副本。
- Redis 只保存可过期或可重建数据。
- OpenSearch 索引可从 PostgreSQL 重建。
- 镜像使用最小运行时和非 root 用户。
- 密钥和配置通过环境注入或密钥管理服务提供。
- 生产环境、预发布环境和开发环境隔离。

扩容顺序：

```text
模块化单体
  ↓
API、Worker、数据库、缓存、搜索和队列分别扩容
  ↓
根据监控确认瓶颈
  ↓
独立拆分搜索、支付、交付或 AI 服务
  ↓
按区域建立多区域读写和灾备能力
```

## 16. 监控与运维

使用 OpenTelemetry 统一采集：

- HTTP 请求延迟、吞吐和错误率。
- 数据库查询和连接池指标。
- Redis 命中率和限流结果。
- 队列积压、处理延迟、重试和死信数量。
- 支付回调延迟和失败率。
- 数字商品交付成功率。
- 实物物流同步状态。
- AI 自动下单授权失败和规则拦截数量。

订单、支付、库存和交付日志需要使用关联 ID 串联，敏感字段必须脱敏。重要审计日志应使用只追加存储或等效防篡改机制。

## 17. 测试方案

### 单元测试

覆盖每个领域模块的实体、值对象、权限判断、金额计算、库存规则和状态转换。

### 集成测试

使用隔离的 PostgreSQL、Redis、OpenSearch 和消息队列测试环境，验证事务、迁移、索引和 Worker 行为。

### 契约测试

验证 OpenAPI 请求和响应、支付服务商回调、物流回调和对象存储适配器。外部服务使用沙盒或模拟服务。

### 端到端测试

至少覆盖：

- 用户注册和卖家创建。
- 数字商品发布、付款和自动交付。
- 实物商品发布、付款、发货和物流同步。
- 用户手动加购和结算。
- AI 搜索和 AI 加购。
- AI 授权自动下单。
- 退款、取消和争议。

### 一致性和安全测试

- 重复支付回调。
- 重复创建订单。
- 并发扣库存。
- Worker 重试和死信恢复。
- AI 越权调用。
- 价格或库存变更竞态。
- 文件上传安全。
- API 限流和令牌撤销。

### 性能测试

在上线前对商品搜索、商品详情、购物车、结算预览、订单查询和 AI API 执行负载测试，并记录延迟、吞吐、错误率和资源使用情况。

## 18. 交付阶段

### 阶段一：设计和基础工程

建立 Cargo workspace、前端项目、数据库迁移、统一错误、认证基础、OpenAPI 和本地开发环境。

### 阶段二：交易核心

实现商品、SKU、库存、购物车、订单、支付沙盒、数字交付和实物订单流程。

### 阶段三：AI 与治理

实现 AI API、授权范围、购买策略、审计日志、商品检查、举报和管理员能力。

### 阶段四：受控生产

确定首发区域和支付服务商，接入生产支付和物流，完成备份、监控、安全检查和故障演练。

### 阶段五：规模化

根据真实指标扩展数据库、搜索、队列和 Worker；只有在模块达到独立部署收益时才拆分服务。