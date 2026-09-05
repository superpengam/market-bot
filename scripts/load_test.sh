#!/usr/bin/env bash
# Hits search, checkout preview, and order lookup when an API is listening.
# Exits 0 when no server is available so CI and local checks stay non-blocking.
set -euo pipefail

API_BASE_URL="${API_BASE_URL:-http://127.0.0.1:3000}"
SAMPLE_ORDER_ID="${SAMPLE_ORDER_ID:-00000000-0000-4000-8000-000000000001}"
SAMPLE_CART_ID="${SAMPLE_CART_ID:-00000000-0000-4000-8000-000000000002}"

echo "Market Bot load test"
echo "Target: ${API_BASE_URL}"
echo "Would hit: GET /api/v1/products/search, POST /api/v1/checkout/preview, GET /api/v1/orders/{order_id}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is not installed; skipping live requests."
  exit 0
fi

if ! curl -fsS --max-time 2 "${API_BASE_URL}/healthz" >/dev/null 2>&1; then
  echo "No API server at ${API_BASE_URL}; skipping live requests."
  exit 0
fi

errors=0
total=0
latency_sum="0"

measure() {
  local name="$1"
  shift
  local metrics status elapsed
  total=$((total + 1))
  metrics="$(curl -sS -o /dev/null --max-time 10 -w "%{http_code} %{time_total}" "$@" || echo "000 0")"
  status="${metrics%% *}"
  elapsed="${metrics##* }"
  latency_sum="$(awk -v acc="${latency_sum}" -v add="${elapsed}" 'BEGIN { printf "%.6f", acc + add }')"
  if [[ "${status}" != "200" && "${status}" != "400" && "${status}" != "404" ]]; then
    errors=$((errors + 1))
  fi
  echo "${name}: status=${status} latency_s=${elapsed}"
}

measure "search" \
  "${API_BASE_URL}/api/v1/products/search?q=notebook"

measure "checkout_preview" \
  -X POST \
  -H "Content-Type: application/json" \
  -d "{\"cart_id\":\"${SAMPLE_CART_ID}\"}" \
  "${API_BASE_URL}/api/v1/checkout/preview"

measure "orders" \
  "${API_BASE_URL}/api/v1/orders/${SAMPLE_ORDER_ID}"

avg="$(awk -v acc="${latency_sum}" -v n="${total}" 'BEGIN { if (n == 0) print "0"; else printf "%.6f", acc / n }')"
error_rate="$(awk -v e="${errors}" -v n="${total}" 'BEGIN { if (n == 0) print "0"; else printf "%.2f", 100 * e / n }')"

echo "requests=${total} errors=${errors} error_rate_pct=${error_rate} avg_latency_s=${avg}"
exit 0
