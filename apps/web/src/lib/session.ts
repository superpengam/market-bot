export const ACCESS_TOKEN_STORAGE_KEY = "market-bot.access_token";

export function readAccessToken(): string | undefined {
  if (typeof window !== "undefined") {
    const fromSession = window.sessionStorage.getItem(ACCESS_TOKEN_STORAGE_KEY);
    if (fromSession?.trim()) {
      return fromSession.trim();
    }
    const fromLocal = window.localStorage.getItem(ACCESS_TOKEN_STORAGE_KEY);
    if (fromLocal?.trim()) {
      return fromLocal.trim();
    }
  }

  const fromEnv = process.env.NEXT_PUBLIC_DEV_ACCESS_TOKEN?.trim();
  return fromEnv || undefined;
}
