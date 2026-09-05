import Link from "next/link";

export default function HomePage() {
  return (
    <section className="home-ledger">
      <h1 className="display">Market Bot</h1>
      <p className="lede">
        A marketplace for digital files and standard physical goods. People and
        authorized AI clients use the same prices, stock, and checkout rules.
      </p>
      <Link className="button-primary" href="/products">
        Open the catalog
      </Link>
    </section>
  );
}
