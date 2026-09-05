import {
  CART_STORAGE_KEY,
  resolveCartId,
  writeStoredCartId,
} from "@/lib/cartStorage";

const ORIGINAL_DEV_CART_ID = process.env.NEXT_PUBLIC_DEV_CART_ID;

afterEach(() => {
  window.localStorage.clear();
  if (ORIGINAL_DEV_CART_ID === undefined) {
    delete process.env.NEXT_PUBLIC_DEV_CART_ID;
  } else {
    process.env.NEXT_PUBLIC_DEV_CART_ID = ORIGINAL_DEV_CART_ID;
  }
});

test("should_prefer_session_stored_cart_id_over_dev_fixture", () => {
  writeStoredCartId("stored-cart");
  process.env.NEXT_PUBLIC_DEV_CART_ID = "env-cart";

  expect(resolveCartId()).toBe("stored-cart");
});

test("should_use_dev_fixture_cart_id_when_session_has_none", () => {
  process.env.NEXT_PUBLIC_DEV_CART_ID = "env-cart";

  expect(resolveCartId()).toBe("env-cart");
  expect(window.localStorage.getItem(CART_STORAGE_KEY)).toBe("env-cart");
});

test("should_return_null_instead_of_inventing_a_create_cart_path", () => {
  delete process.env.NEXT_PUBLIC_DEV_CART_ID;

  expect(resolveCartId()).toBeNull();
});
