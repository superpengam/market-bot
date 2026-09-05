import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ProductDetail } from "@/features/catalog/ProductDetail";
import { apiClient } from "@/lib/api-client";
import { CART_STORAGE_KEY } from "@/lib/cartStorage";
import type { ProductDetail as ProductDetailData } from "@/lib/types";

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

const mockGet = apiClient.get as jest.Mock;
const mockPost = apiClient.post as jest.Mock;

const ORIGINAL_DEV_CART_ID = process.env.NEXT_PUBLIC_DEV_CART_ID;

function listing(): ProductDetailData {
  return {
    product_id: "11111111-1111-1111-1111-111111111111",
    seller_id: "seller-1",
    store_name: "Field Press",
    title: "Field Notes Pack",
    description: "A pocket notebook.",
    fulfillment_type: "digital",
    price_minor: 1999,
    currency: "USD",
    available_stock: 8,
    status: "published",
    delivery_rules: {
      fulfillment_type: "digital",
      estimated_days_min: 0,
      estimated_days_max: 0,
      digital_method: "file_download",
    },
    refund_rules: {
      refund_window_days: 14,
      is_refundable: true,
      summary: "14 day refund window.",
    },
    variants: [
      {
        variant_id: "22222222-2222-2222-2222-222222222222",
        sku: "NOTES-01",
        price_minor: 1999,
        currency: "USD",
        available_stock: 8,
      },
    ],
  };
}

async function renderReadyDetail() {
  mockGet.mockResolvedValue(listing());
  render(<ProductDetail productId="11111111-1111-1111-1111-111111111111" />);
  await waitFor(() => {
    expect(screen.getByRole("button", { name: "Add to cart" })).toBeEnabled();
  });
}

beforeEach(() => {
  mockGet.mockReset();
  mockPost.mockReset();
  window.localStorage.clear();
  if (ORIGINAL_DEV_CART_ID === undefined) {
    delete process.env.NEXT_PUBLIC_DEV_CART_ID;
  } else {
    process.env.NEXT_PUBLIC_DEV_CART_ID = ORIGINAL_DEV_CART_ID;
  }
});

test("should_add_to_existing_session_cart_with_contract_fields_only", async () => {
  window.localStorage.setItem(CART_STORAGE_KEY, "cart-session");
  mockPost.mockResolvedValue({ cart_id: "cart-session", items: [] });
  await renderReadyDetail();

  fireEvent.click(screen.getByRole("button", { name: "Add to cart" }));

  await waitFor(() => {
    expect(mockPost).toHaveBeenCalledTimes(1);
  });
  expect(mockPost).toHaveBeenCalledWith("/carts/cart-session/items", {
    product_id: "11111111-1111-1111-1111-111111111111",
    variant_id: "22222222-2222-2222-2222-222222222222",
    quantity: 1,
    source: "user",
  });
  expect(Object.keys(mockPost.mock.calls[0][1]).sort()).toEqual([
    "product_id",
    "quantity",
    "source",
    "variant_id",
  ]);
});

test("should_use_dev_fixture_cart_id_instead_of_posting_to_carts", async () => {
  process.env.NEXT_PUBLIC_DEV_CART_ID = "dev-cart";
  mockPost.mockResolvedValue({ cart_id: "dev-cart", items: [] });
  await renderReadyDetail();

  fireEvent.click(screen.getByRole("button", { name: "Add to cart" }));

  await waitFor(() => {
    expect(mockPost).toHaveBeenCalledTimes(1);
  });
  expect(mockPost.mock.calls[0][0]).toBe("/carts/dev-cart/items");
  expect(mockPost.mock.calls.some((call) => call[0] === "/carts")).toBe(false);
});

test("should_not_invent_post_carts_when_no_session_cart_exists", async () => {
  delete process.env.NEXT_PUBLIC_DEV_CART_ID;
  await renderReadyDetail();

  fireEvent.click(screen.getByRole("button", { name: "Add to cart" }));

  await waitFor(() => {
    expect(screen.getByRole("alert")).toHaveTextContent(
      "No cart is available for this session.",
    );
  });
  expect(mockPost).not.toHaveBeenCalled();
});
