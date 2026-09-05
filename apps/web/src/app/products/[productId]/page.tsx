import { ProductDetail } from "@/features/catalog/ProductDetail";

type ProductDetailPageProps = {
  params: Promise<{ productId: string }>;
};

export default async function ProductDetailPage({
  params,
}: ProductDetailPageProps) {
  const { productId } = await params;
  return <ProductDetail productId={productId} />;
}
