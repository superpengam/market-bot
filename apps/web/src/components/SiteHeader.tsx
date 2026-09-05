import { Basket, Storefront, SquaresFour } from "@phosphor-icons/react/dist/ssr";
import Link from "next/link";

export function SiteHeader() {
  return (
    <header className="masthead">
      <Link className="wordmark" href="/">
        Market Bot
      </Link>
      <nav aria-label="Primary">
        <Link href="/products">
          <SquaresFour size={16} weight="regular" aria-hidden="true" />
          Catalog
        </Link>
        <Link href="/cart">
          <Basket size={16} weight="regular" aria-hidden="true" />
          Cart
        </Link>
        <Link href="/seller/products/new">
          <Storefront size={16} weight="regular" aria-hidden="true" />
          Sell
        </Link>
      </nav>
    </header>
  );
}
