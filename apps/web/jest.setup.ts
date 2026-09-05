import "@testing-library/jest-dom";

if (typeof globalThis.fetch !== "function") {
  globalThis.fetch = (() => {
    throw new Error("fetch is not stubbed");
  }) as typeof fetch;
}
