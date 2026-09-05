import { formatMoney } from "@/lib/money";
import {
  fulfillmentStatusLabel,
  fulfillmentTypeLabel,
  orderStatusLabel,
  paymentStatusLabel,
  shipmentStatusLabel,
} from "@/lib/statusMaps";
import type { OrderDetail } from "@/lib/types";

export type OrderStatusProps = {
  order: OrderDetail;
};

export function OrderStatus({ order }: OrderStatusProps) {
  return (
    <article className="stack-lg">
      <header className="section-heading">
        <p className="meta money">Order {order.order_id}</p>
        <h1 className="display">Order status</h1>
      </header>

      <dl className="status-grid">
        <div>
          <dt>Order</dt>
          <dd>{orderStatusLabel(order.order_status)}</dd>
        </div>
        <div>
          <dt>Payment</dt>
          <dd>{paymentStatusLabel(order.payment_status)}</dd>
        </div>
        <div>
          <dt>Fulfillment</dt>
          <dd>{fulfillmentStatusLabel(order.fulfillment_status)}</dd>
        </div>
        <div>
          <dt>Shipment</dt>
          <dd>
            {order.shipment_status
              ? shipmentStatusLabel(order.shipment_status)
              : "No shipment"}
          </dd>
        </div>
      </dl>

      <ol className="editorial-list">
        {order.items.map((item) => (
          <li key={item.order_item_id} className="editorial-row">
            <div className="editorial-main">
              <h2>{item.title}</h2>
              <p className="meta">
                {fulfillmentTypeLabel(item.fulfillment_type)}, qty {item.quantity}
              </p>
            </div>
            <p className="money">
              {formatMoney(item.unit_price_minor, item.currency)}
            </p>
          </li>
        ))}
      </ol>

      <dl className="totals">
        <div>
          <dt>Subtotal</dt>
          <dd className="money">
            {formatMoney(order.subtotal_minor, order.currency)}
          </dd>
        </div>
        <div>
          <dt>Shipping</dt>
          <dd className="money">
            {formatMoney(order.shipping_fee_minor, order.currency)}
          </dd>
        </div>
        <div>
          <dt>Tax</dt>
          <dd className="money">
            {formatMoney(order.tax_minor, order.currency)}
          </dd>
        </div>
        <div className="totals-total">
          <dt>Total</dt>
          <dd className="money">
            {formatMoney(order.total_minor, order.currency)}
          </dd>
        </div>
      </dl>
    </article>
  );
}
