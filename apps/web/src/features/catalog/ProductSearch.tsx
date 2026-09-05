"use client";

import { FormEvent, useCallback, useState } from "react";
import { EmptyState, ErrorState, LoadingState } from "@/components/FeedbackStates";
import { apiClient } from "@/lib/api-client";
import { useRemoteData } from "@/lib/useRemoteData";
import type {
  FulfillmentType,
  ProductSearchPage,
  ProductSearchQuery,
} from "@/lib/types";
import { ProductCard } from "./ProductCard";

const EMPTY_QUERY: ProductSearchQuery = {
  q: "",
  category_id: "",
  currency: "",
  min_price_minor: undefined,
  max_price_minor: undefined,
  fulfillment_type: undefined,
};

function queryKey(query: ProductSearchQuery): string {
  return JSON.stringify({
    q: query.q ?? "",
    category_id: query.category_id ?? "",
    currency: query.currency ?? "",
    min_price_minor: query.min_price_minor ?? "",
    max_price_minor: query.max_price_minor ?? "",
    fulfillment_type: query.fulfillment_type ?? "",
  });
}

export function ProductSearch() {
  const [draft, setDraft] = useState<ProductSearchQuery>(EMPTY_QUERY);
  const [applied, setApplied] = useState<ProductSearchQuery>(EMPTY_QUERY);

  const loader = useCallback(async () => {
    const result = await apiClient.get<ProductSearchPage>("/products/search", {
      query: {
        q: applied.q,
        category_id: applied.category_id,
        currency: applied.currency,
        min_price_minor: applied.min_price_minor,
        max_price_minor: applied.max_price_minor,
        fulfillment_type: applied.fulfillment_type,
      },
    });
    const items = applied.fulfillment_type
      ? result.items.filter(
          (item) => item.fulfillment_type === applied.fulfillment_type,
        )
      : result.items;
    return { ...result, items };
  }, [applied]);

  const { data: page, error, isLoading, reload, beginLoad } = useRemoteData(
    loader,
    queryKey(applied),
  );

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    beginLoad();
    setApplied({ ...draft });
  }

  return (
    <section className="stack-lg">
      <header className="section-heading">
        <h1 className="display">Catalog</h1>
        <p className="lede">
          Filter by words, category, price, currency, and delivery type. The
          same search contract is used by people and AI clients.
        </p>
      </header>

      <form className="filter-grid" onSubmit={handleSubmit}>
        <label>
          Keywords
          <input
            name="q"
            value={draft.q ?? ""}
            onChange={(event) =>
              setDraft((current) => ({ ...current, q: event.target.value }))
            }
          />
        </label>
        <label>
          Category
          <input
            name="category_id"
            value={draft.category_id ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                category_id: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Currency
          <input
            name="currency"
            maxLength={3}
            value={draft.currency ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                currency: event.target.value.toUpperCase(),
              }))
            }
          />
        </label>
        <label>
          Min price (minor units)
          <input
            name="min_price_minor"
            inputMode="numeric"
            value={draft.min_price_minor ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                min_price_minor: event.target.value
                  ? Number.parseInt(event.target.value, 10)
                  : undefined,
              }))
            }
          />
        </label>
        <label>
          Max price (minor units)
          <input
            name="max_price_minor"
            inputMode="numeric"
            value={draft.max_price_minor ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                max_price_minor: event.target.value
                  ? Number.parseInt(event.target.value, 10)
                  : undefined,
              }))
            }
          />
        </label>
        <label>
          Delivery type
          <select
            name="fulfillment_type"
            value={draft.fulfillment_type ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                fulfillment_type: (event.target.value || undefined) as
                  | FulfillmentType
                  | undefined,
              }))
            }
          >
            <option value="">Any</option>
            <option value="digital">Digital delivery</option>
            <option value="physical_standard">Physical shipment</option>
          </select>
        </label>
        <button type="submit" className="button-primary">
          Apply filters
        </button>
      </form>

      {isLoading ? <LoadingState label="Loading catalog" /> : null}
      {!isLoading && error ? <ErrorState error={error} onRetry={reload} /> : null}
      {!isLoading && !error && page && page.items.length === 0 ? (
        <EmptyState
          title="No listings match"
          body="Change a filter or clear the keyword. Published items with available stock appear here."
        />
      ) : null}
      {!isLoading && !error && page && page.items.length > 0 ? (
        <div className="editorial-list">
          {page.items.map((product) => (
            <ProductCard key={product.product_id} product={product} />
          ))}
        </div>
      ) : null}
    </section>
  );
}
