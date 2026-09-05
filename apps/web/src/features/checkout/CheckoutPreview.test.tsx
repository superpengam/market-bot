import { render, screen } from "@testing-library/react";
import type { CheckoutPreview as CheckoutPreviewData } from "@/lib/types";
import { CheckoutPreview } from "./CheckoutPreview";

function usdPreview(
  overrides: Partial<CheckoutPreviewData> = {},
): CheckoutPreviewData {
  return {
    items: [
      {
        product_id: "11111111-1111-1111-1111-111111111111",
        variant_id: "22222222-2222-2222-2222-222222222222",
        title: "Field Notes Pack",
        quantity: 1,
        snapshot_unit_price_minor: 1999,
        current_unit_price_minor: 1999,
        currency: "USD",
        fulfillment_type: "digital",
        available_stock: 12,
        source: "user",
        digital_delivery_method: "file_download",
      },
      {
        product_id: "33333333-3333-3333-3333-333333333333",
        variant_id: "44444444-4444-4444-4444-444444444444",
        title: "Press Cloth",
        quantity: 2,
        snapshot_unit_price_minor: 4500,
        current_unit_price_minor: 4500,
        currency: "USD",
        fulfillment_type: "physical_standard",
        available_stock: 4,
        source: "ai",
        shipping: {
          destination_region: "US-CA",
          method: "standard_ground",
          estimated_days_min: 3,
          estimated_days_max: 7,
        },
      },
    ],
    subtotal_minor: 10999,
    shipping_fee_minor: 799,
    tax_minor: 880,
    total_minor: 12678,
    currency: "USD",
    expires_at: "2026-09-03T14:30:00.000Z",
    requires_price_reconfirm: false,
    inventory_is_available: true,
    payment_provider_status: "not_started",
    ...overrides,
  };
}

function jpyPreview(): CheckoutPreviewData {
  return {
    items: [
      {
        product_id: "55555555-5555-5555-5555-555555555555",
        variant_id: "66666666-6666-6666-6666-666666666666",
        title: "Tokyo Print License",
        quantity: 1,
        snapshot_unit_price_minor: 2500,
        current_unit_price_minor: 2500,
        currency: "JPY",
        fulfillment_type: "digital",
        available_stock: 80,
        source: "user",
        digital_delivery_method: "license_key",
      },
    ],
    subtotal_minor: 2500,
    shipping_fee_minor: 0,
    tax_minor: 250,
    total_minor: 2750,
    currency: "JPY",
    expires_at: "2026-09-03T16:00:00.000Z",
    requires_price_reconfirm: false,
    inventory_is_available: true,
    payment_provider_status: "not_started",
  };
}

test("should_display_usd_snapshot_price_shipping_tax_total_and_expiry", () => {
  render(<CheckoutPreview preview={usdPreview()} />);

  expect(screen.getByText("USD 19.99")).toBeInTheDocument();
  expect(screen.getByLabelText("Shipping")).toHaveTextContent("USD 7.99");
  expect(screen.getByLabelText("Tax")).toHaveTextContent("USD 8.80");
  expect(screen.getByLabelText("Total")).toHaveTextContent("USD 126.78");
  expect(screen.getByLabelText("Preview expiry")).toHaveTextContent(
    "2026-09-03T14:30:00.000Z",
  );
});

test("should_display_jpy_amounts_without_fractional_units", () => {
  render(<CheckoutPreview preview={jpyPreview()} />);

  expect(screen.getAllByText("JPY 2,500").length).toBeGreaterThanOrEqual(1);
  expect(screen.getByLabelText("Shipping")).toHaveTextContent("JPY 0");
  expect(screen.getByLabelText("Tax")).toHaveTextContent("JPY 250");
  expect(screen.getByLabelText("Total")).toHaveTextContent("JPY 2,750");
});

test("should_require_reconfirm_when_snapshot_price_changed", () => {
  const preview = usdPreview({
    requires_price_reconfirm: true,
    items: [
      {
        product_id: "11111111-1111-1111-1111-111111111111",
        variant_id: "22222222-2222-2222-2222-222222222222",
        title: "Field Notes Pack",
        quantity: 1,
        snapshot_unit_price_minor: 1999,
        current_unit_price_minor: 2499,
        currency: "USD",
        fulfillment_type: "digital",
        available_stock: 12,
        source: "user",
        digital_delivery_method: "file_download",
      },
    ],
  });

  render(<CheckoutPreview preview={preview} />);

  expect(screen.getByRole("alert")).toHaveTextContent(
    "Price changed. Reconfirm before paying.",
  );
  expect(screen.getByText("USD 19.99")).toBeInTheDocument();
  expect(screen.getByText("USD 24.99")).toBeInTheDocument();
});

test("should_show_digital_delivery_and_physical_shipping_details", () => {
  render(<CheckoutPreview preview={usdPreview()} />);

  expect(screen.getByText("File download")).toBeInTheDocument();
  expect(screen.getByText("US-CA")).toBeInTheDocument();
  expect(screen.getByText("Standard ground")).toBeInTheDocument();
});

test("should_display_inventory_and_payment_provider_redirect_status", () => {
  render(
    <CheckoutPreview
      preview={usdPreview({
        inventory_is_available: false,
        payment_provider_status: "redirecting",
      })}
    />,
  );

  expect(screen.getByLabelText("Inventory")).toHaveTextContent("Unavailable");
  expect(screen.getByLabelText("Payment provider")).toHaveTextContent(
    "Redirecting to payment provider",
  );
});
