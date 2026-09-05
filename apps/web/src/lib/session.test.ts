import { ACCESS_TOKEN_STORAGE_KEY, readAccessToken } from "@/lib/session";

const ORIGINAL_DEV_TOKEN = process.env.NEXT_PUBLIC_DEV_ACCESS_TOKEN;

afterEach(() => {
  window.sessionStorage.clear();
  window.localStorage.clear();
  if (ORIGINAL_DEV_TOKEN === undefined) {
    delete process.env.NEXT_PUBLIC_DEV_ACCESS_TOKEN;
  } else {
    process.env.NEXT_PUBLIC_DEV_ACCESS_TOKEN = ORIGINAL_DEV_TOKEN;
  }
});

test("should_read_access_token_from_session_then_local_then_dev_fixture", () => {
  process.env.NEXT_PUBLIC_DEV_ACCESS_TOKEN = "env-token";
  expect(readAccessToken()).toBe("env-token");

  window.localStorage.setItem(ACCESS_TOKEN_STORAGE_KEY, "local-token");
  expect(readAccessToken()).toBe("local-token");

  window.sessionStorage.setItem(ACCESS_TOKEN_STORAGE_KEY, "session-token");
  expect(readAccessToken()).toBe("session-token");
});
