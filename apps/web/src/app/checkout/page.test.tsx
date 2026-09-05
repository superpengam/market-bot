import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import CheckoutPage from "@/app/checkout/page";
import { apiClient } from "@/lib/api-client";
import { CART_STORAGE_KEY } from "@/lib/cartStorage";
import { followPaymentRedirect } from "@/lib/orderWrite";
import type { CheckoutPreview, OrderDetail } from "@/lib/types";

jest.mock("../../lib/api-client", () => {
  const actual = jest.requireActual("../../lib/api-client");
  return {
    ...actual,
    apiClient: {
      get: jest.fn(),
      post: jest.fn(),
    },
  };
});

jest.mock("../../lib/orderWrite", () => {
  const actual = jest.requireActual("../../lib/orderWrite");
  return {
    ...actual,
    followPaymentRedirect: jest.fn(),
  };
});

const mockFollowPaymentRedirect = followPaymentRedirect as jest.Mock;

const mockPost = apiClient.post as jest.Mock;

function preview(): CheckoutPreview {
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
    ],
    subtotal_minor: 1999,
    shipping_fee_minor: 0,
    tax_minor: 160,
    total_minor: 2159,
    currency: "USD",
    expires_at: "2026-09-03T14:30:00.000Z",
    requires_price_reconfirm: false,
    inventory_is_available: true,
    payment_provider_status: "not_started",
  };
}

function order(overrides: Partial<OrderDetail> = {}): OrderDetail {
  return {
    order_id: "ord-1",
    order_status: "pending_payment",
    payment_status: "created",
    fulfillment_status: "pending",
    shipment_status: null,
    items: [],
    subtotal_minor: 1999,
    shipping_fee_minor: 0,
    tax_minor: 160,
    total_minor: 2159,
    currency: "USD",
    created_at: "2026-09-03T14:00:00.000Z",
    ...overrides,
  };
}

beforeEach(() => {
  mockPost.mockReset();
  mockFollowPaymentRedirect.mockReset();
  mockFollowPaymentRedirect.mockReturnValue(false);
  window.localStorage.clear();
  window.sessionStorage.clear();
  window.localStorage.setItem(CART_STORAGE_KEY, "cart-1");
});

async function renderReadyCheckout(created: OrderDetail = order()) {
  mockPost.mockImplementation(async (path: string) => {
    if (path === "/checkout/preview") {
      return preview();
    }
    if (path === "/orders") {
      return created;
    }
    throw new Error(`unexpected path ${path}`);
  });
  render(<CheckoutPage />);
  await waitFor(() => {
    expect(screen.getByRole("button", { name: "Place order" })).toBeEnabled();
  });
}

test("should_label_place_order_without_claiming_a_payment_provider", async () => {
  await renderReadyCheckout();

  expect(
    screen.queryByRole("button", { name: "Continue to payment provider" }),
  ).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Place order" })).toBeInTheDocument();
});

test("should_reuse_one_idempotency_key_when_place_order_is_retried", async () => {
  mockPost.mockImplementation(async (path: string) => {
    if (path === "/checkout/preview") {
      return preview();
    }
    if (path === "/orders") {
      throw new Error("network dropped");
    }
    throw new Error(`unexpected path ${path}`);
  });
  render(<CheckoutPage />);
  await waitFor(() => {
    expect(screen.getByRole("button", { name: "Place order" })).toBeEnabled();
  });

  fireEvent.click(screen.getByRole("button", { name: "Place order" }));
  await waitFor(() => {
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });
  fireEvent.click(screen.getByRole("button", { name: "Place order" }));
  await waitFor(() => {
    expect(
      mockPost.mock.calls.filter((call) => call[0] === "/orders"),
    ).toHaveLength(2);
  });

  const orderCalls = mockPost.mock.calls.filter((call) => call[0] === "/orders");
  expect(orderCalls[0][2]).toEqual(
    expect.objectContaining({ idempotencyKey: expect.any(String) }),
  );
  expect(orderCalls[0][2].idempotencyKey).toBe(orderCalls[1][2].idempotencyKey);
});

test("should_follow_provider_redirect_url_after_creating_an_order", async () => {
  mockFollowPaymentRedirect.mockReturnValue(true);
  await renderReadyCheckout(
    order({ payment_redirect_url: "https://pay.example/sandbox" }),
  );

  fireEvent.click(screen.getByRole("button", { name: "Place order" }));

  await waitFor(() => {
    expect(mockFollowPaymentRedirect).toHaveBeenCalledWith(
      expect.objectContaining({
        payment_redirect_url: "https://pay.example/sandbox",
      }),
    );
  });
  expect(
    screen.queryByRole("link", { name: "View order status" }),
  ).not.toBeInTheDocument();
});
