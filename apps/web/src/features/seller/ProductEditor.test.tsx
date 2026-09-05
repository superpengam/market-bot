import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ProductEditor } from "@/features/seller/ProductEditor";
import { apiClient } from "@/lib/api-client";

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

const mockPost = apiClient.post as jest.Mock;

beforeEach(() => {
  mockPost.mockReset();
  mockPost.mockResolvedValue({
    product_id: "33333333-3333-3333-3333-333333333333",
  });
});

test("should_create_listing_on_seller_products_path", async () => {
  render(<ProductEditor />);

  fireEvent.change(screen.getByLabelText("Title"), {
    target: { value: "Field Notes Pack" },
  });
  fireEvent.change(screen.getByLabelText("Description"), {
    target: { value: "A pocket notebook." },
  });
  fireEvent.click(screen.getByRole("button", { name: "Submit listing" }));

  await waitFor(() => {
    expect(mockPost).toHaveBeenCalledTimes(1);
  });
  expect(mockPost.mock.calls[0][0]).toBe("/seller/products");
  expect(mockPost.mock.calls[0][1]).toEqual(
    expect.objectContaining({
      title: "Field Notes Pack",
      description: "A pocket notebook.",
      fulfillment_type: "digital",
      price_minor: 1999,
      currency: "USD",
      available_stock: 10,
    }),
  );
  expect(mockPost.mock.calls[0][1]).not.toHaveProperty("seller_id");
});
