const ZERO_DECIMAL_CURRENCIES = new Set([
  "BIF",
  "CLP",
  "DJF",
  "GNF",
  "ISK",
  "JPY",
  "KMF",
  "KRW",
  "PYG",
  "RWF",
  "UGX",
  "VND",
  "VUV",
  "XAF",
  "XOF",
  "XPF",
]);

const THREE_DECIMAL_CURRENCIES = new Set(["BHD", "JOD", "KWD", "OMR", "TND"]);

export function currencyExponent(currency: string): 0 | 2 | 3 {
  if (ZERO_DECIMAL_CURRENCIES.has(currency)) {
    return 0;
  }
  if (THREE_DECIMAL_CURRENCIES.has(currency)) {
    return 3;
  }
  return 2;
}

export function minorFactor(currency: string): 1 | 100 | 1000 {
  const exponent = currencyExponent(currency);
  if (exponent === 0) {
    return 1;
  }
  if (exponent === 3) {
    return 1000;
  }
  return 100;
}

function groupInteger(value: number): string {
  const digits = Math.trunc(value).toString();
  return digits.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

// Why: money stays in integer minor units so UI formatting cannot introduce float rounding.
export function formatMoney(amountMinor: number, currency: string): string {
  if (!Number.isInteger(amountMinor)) {
    throw new Error("amount_minor must be an integer");
  }
  if (amountMinor < 0) {
    throw new Error("amount_minor cannot be negative");
  }
  if (!/^[A-Z]{3}$/.test(currency)) {
    throw new Error("currency must be an ISO 4217 code");
  }

  const factor = minorFactor(currency);
  if (factor === 1) {
    return `${currency} ${groupInteger(amountMinor)}`;
  }

  const major = Math.trunc(amountMinor / factor);
  const fraction = amountMinor % factor;
  const fractionDigits = factor === 1000 ? 3 : 2;
  return `${currency} ${groupInteger(major)}.${String(fraction).padStart(fractionDigits, "0")}`;
}

export function parseMajorToMinor(value: string, currency: string): number {
  const trimmed = value.trim();
  const factor = minorFactor(currency);

  if (factor === 1) {
    if (!/^\d+$/.test(trimmed)) {
      throw new Error("zero-decimal currencies accept whole numbers only");
    }
    return Number.parseInt(trimmed, 10);
  }

  const parts = trimmed.split(".");
  if (parts.length > 2 || !parts[0] || !/^\d+$/.test(parts[0])) {
    throw new Error("price must be a decimal amount");
  }

  const fraction = parts[1] ?? "";
  if (fraction && !/^\d+$/.test(fraction)) {
    throw new Error("price fraction must be digits");
  }

  const exponent = factor === 1000 ? 3 : 2;
  if (fraction.length > exponent) {
    throw new Error("price has too many fractional digits");
  }

  const padded = fraction.padEnd(exponent, "0");
  return Number.parseInt(parts[0], 10) * factor + Number.parseInt(padded, 10);
}
