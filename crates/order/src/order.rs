use market_bot_shared::{
    FulfillmentType, Money, OrderId, OrderItemId, ProductId, ProductVariantId, SellerId, UserId,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Order {
    id: OrderId,
    buyer_id: UserId,
    items: Vec<OrderItem>,
    subtotal: Money,
    shipping_fee: Money,
    tax: Money,
    total: Money,
    status: OrderStatus,
}

impl Order {
    pub fn new(
        buyer_id: UserId,
        items: Vec<OrderItem>,
        shipping_fee: Money,
        tax: Money,
    ) -> Result<Self, OrderError> {
        let first_item = items.first().ok_or(OrderError::EmptyOrder)?;
        let currency = first_item.unit_price.currency().clone();
        let mut subtotal = Money::new(0, currency.clone()).map_err(OrderError::Money)?;

        for item in &items {
            let line_total = item
                .unit_price
                .clone()
                .checked_mul(item.quantity)
                .map_err(OrderError::Money)?;
            subtotal = subtotal
                .checked_add(line_total)
                .map_err(OrderError::Money)?;
        }

        let total = subtotal
            .clone()
            .checked_add(shipping_fee.clone())
            .and_then(|value| value.checked_add(tax.clone()))
            .map_err(OrderError::Money)?;

        Ok(Self {
            id: OrderId::new(),
            buyer_id,
            items,
            subtotal,
            shipping_fee,
            tax,
            total,
            status: OrderStatus::Draft,
        })
    }

    pub const fn id(&self) -> OrderId {
        self.id
    }

    pub const fn buyer_id(&self) -> UserId {
        self.buyer_id
    }

    pub fn items(&self) -> &[OrderItem] {
        &self.items
    }

    pub const fn subtotal(&self) -> &Money {
        &self.subtotal
    }

    pub const fn shipping_fee(&self) -> &Money {
        &self.shipping_fee
    }

    pub const fn tax(&self) -> &Money {
        &self.tax
    }

    pub const fn total(&self) -> &Money {
        &self.total
    }

    pub const fn status(&self) -> OrderStatus {
        self.status
    }

    pub fn transition_to(&mut self, next: OrderStatus) -> Result<(), OrderError> {
        if !self.status.can_transition_to(next) {
            return Err(OrderError::InvalidStatusTransition {
                from: self.status,
                to: next,
            });
        }

        self.status = next;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OrderItem {
    id: OrderItemId,
    product_id: ProductId,
    variant_id: ProductVariantId,
    seller_id: SellerId,
    title: String,
    unit_price: Money,
    quantity: u64,
    fulfillment_type: FulfillmentType,
}

impl OrderItem {
    pub fn new(
        product_id: ProductId,
        variant_id: ProductVariantId,
        seller_id: SellerId,
        title: String,
        unit_price: Money,
        quantity: u64,
        fulfillment_type: FulfillmentType,
    ) -> Result<Self, OrderError> {
        if quantity == 0 {
            return Err(OrderError::InvalidQuantity);
        }
        let title = title.trim().to_owned();
        if title.is_empty() {
            return Err(OrderError::BlankTitle);
        }

        Ok(Self {
            id: OrderItemId::new(),
            product_id,
            variant_id,
            seller_id,
            title,
            unit_price,
            quantity,
            fulfillment_type,
        })
    }

    pub const fn id(&self) -> OrderItemId {
        self.id
    }

    pub const fn product_id(&self) -> ProductId {
        self.product_id
    }

    pub const fn variant_id(&self) -> ProductVariantId {
        self.variant_id
    }

    pub const fn seller_id(&self) -> SellerId {
        self.seller_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub const fn unit_price(&self) -> &Money {
        &self.unit_price
    }

    pub const fn quantity(&self) -> u64 {
        self.quantity
    }

    pub const fn fulfillment_type(&self) -> FulfillmentType {
        self.fulfillment_type
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OrderStatus {
    Draft,
    PendingConfirmation,
    PendingPayment,
    PaymentProcessing,
    Paid,
    FulfillmentProcessing,
    Shipped,
    Delivered,
    Completed,
    CancellationRequested,
    Cancelled,
    RefundProcessing,
    Refunded,
    DisputeProcessing,
}

impl OrderStatus {
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::PendingConfirmation | Self::Cancelled)
                | (
                    Self::PendingConfirmation,
                    Self::PendingPayment | Self::Cancelled
                )
                | (
                    Self::PendingPayment,
                    Self::PaymentProcessing | Self::Cancelled
                )
                | (Self::PaymentProcessing, Self::Paid | Self::PendingPayment)
                | (
                    Self::Paid,
                    Self::FulfillmentProcessing | Self::RefundProcessing | Self::DisputeProcessing
                )
                | (
                    Self::FulfillmentProcessing,
                    Self::Shipped | Self::Delivered | Self::RefundProcessing
                )
                | (
                    Self::Shipped,
                    Self::Delivered | Self::RefundProcessing | Self::DisputeProcessing
                )
                | (
                    Self::Delivered,
                    Self::Completed | Self::RefundProcessing | Self::DisputeProcessing
                )
                | (
                    Self::Completed,
                    Self::RefundProcessing | Self::DisputeProcessing
                )
                | (Self::CancellationRequested, Self::Cancelled)
                | (Self::RefundProcessing, Self::Refunded)
                | (
                    Self::DisputeProcessing,
                    Self::RefundProcessing | Self::Completed
                )
        )
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum OrderError {
    #[error("order must contain at least one item")]
    EmptyOrder,
    #[error("order item quantity must be greater than zero")]
    InvalidQuantity,
    #[error("order item title cannot be blank")]
    BlankTitle,
    #[error("order status transition from {from:?} to {to:?} is invalid")]
    InvalidStatusTransition { from: OrderStatus, to: OrderStatus },
    #[error("order money operation failed: {0}")]
    Money(#[source] market_bot_shared::MoneyError),
}
