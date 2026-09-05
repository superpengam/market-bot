import Link from "next/link";
import { formatMoney } from "@/lib/money";
import { fulfillmentTypeLabel, stockHint } from "@/lib/statusMaps";
import type { ProductSearchResult } from "@/lib/types";

export type ProductCardProps = {
  product: ProductSearchResult;
};

export function ProductCard({ product }: ProductCardProps) {
  return (
    <article className="editorial-row">
      <div className="editorial-main">
        <h2>
          <Link href={`/products/${product.product_id}`}>{product.title}</Link>
        </h2>
        <p className="meta">
          {fulfillmentTypeLabel(product.fulfillment_type)},{" "}
          {stockHint(product.available_stock)}
        </p>
      </div>
      <div className="editorial-aside">
        <p className="money">
          {formatMoney(product.price_minor, product.currency)}
        </p>
        <p className="meta">{product.available_stock} available</p>
      </div>
    </article>
  );
}
