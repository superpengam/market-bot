import { formatMoney } from "@/lib/money";
import {
  digitalDeliveryMethodLabel,
  fulfillmentTypeLabel,
  paymentProviderStatusLabel,
  shippingMethodLabel,
  stockHint,
} from "@/lib/statusMaps";
import type { CheckoutPreview as CheckoutPreviewData } from "@/lib/types";

export type CheckoutPreviewProps = {
  preview: CheckoutPreviewData;
  onReconfirm?: () => void;
};

export function CheckoutPreview({
  preview,
  onReconfirm,
}: CheckoutPreviewProps) {
  return (
    <section className="stack-lg" aria-labelledby="checkout-preview-title">
      <header className="section-heading">
        <h2 id="checkout-preview-title" className="display">
          Checkout preview
        </h2>
        <p className="lede">
          Prices, stock, shipping, and tax are frozen until the expiry time.
        </p>
      </header>

      {preview.requires_price_reconfirm ? (
        <p className="banner banner-alert" role="alert">
          Price changed. Reconfirm before paying.
          {onReconfirm ? (
            <button type="button" className="text-action" onClick={onReconfirm}>
              Reconfirm prices
            </button>
          ) : null}
        </p>
      ) : null}

      <ol className="editorial-list">
        {preview.items.map((item) => {
          const lineKey = `${item.product_id}:${item.variant_id}`;
          const priceChanged =
            item.snapshot_unit_price_minor !== item.current_unit_price_minor;

          return (
            <li key={lineKey} className="editorial-row">
              <div className="editorial-main">
                <h3>{item.title}</h3>
                <p className="meta">
                  {fulfillmentTypeLabel(item.fulfillment_type)}, qty{" "}
                  {item.quantity}, {stockHint(item.available_stock)}
                </p>
                {item.fulfillment_type === "digital" &&
                item.digital_delivery_method ? (
                  <p>{digitalDeliveryMethodLabel(item.digital_delivery_method)}</p>
                ) : null}
                {item.fulfillment_type === "physical_standard" &&
                item.shipping ? (
                  <p className="shipping-detail">
                    <span>{item.shipping.destination_region}</span>
                    <span>{shippingMethodLabel(item.shipping.method)}</span>
                    <span>
                      {item.shipping.estimated_days_min}-
                      {item.shipping.estimated_days_max} days
                    </span>
                  </p>
                ) : null}
              </div>
              <div className="editorial-aside">
                <p className="money">{formatMoney(item.snapshot_unit_price_minor, item.currency)}</p>
                <p className="meta">Snapshot price</p>
                {priceChanged ? (
                  <p className="money money-changed">
                    {formatMoney(item.current_unit_price_minor, item.currency)}
                  </p>
                ) : null}
              </div>
            </li>
          );
        })}
      </ol>

      <dl className="totals">
        <div aria-label="Subtotal">
          <dt>Subtotal</dt>
          <dd className="money">
            {formatMoney(preview.subtotal_minor, preview.currency)}
          </dd>
        </div>
        <div aria-label="Shipping">
          <dt>Shipping</dt>
          <dd className="money">
            {formatMoney(preview.shipping_fee_minor, preview.currency)}
          </dd>
        </div>
        <div aria-label="Tax">
          <dt>Tax</dt>
          <dd className="money">
            {formatMoney(preview.tax_minor, preview.currency)}
          </dd>
        </div>
        <div aria-label="Total" className="totals-total">
          <dt>Total</dt>
          <dd className="money">
            {formatMoney(preview.total_minor, preview.currency)}
          </dd>
        </div>
        <div aria-label="Preview expiry">
          <dt>Expires</dt>
          <dd className="money">{preview.expires_at}</dd>
        </div>
        <div aria-label="Inventory">
          <dt>Inventory</dt>
          <dd>{preview.inventory_is_available ? "Available" : "Unavailable"}</dd>
        </div>
        <div aria-label="Payment provider">
          <dt>Payment provider</dt>
          <dd>{paymentProviderStatusLabel(preview.payment_provider_status)}</dd>
        </div>
      </dl>
    </section>
  );
}
