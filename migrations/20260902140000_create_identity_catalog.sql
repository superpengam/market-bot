-- Identity and seller records are kept separate so one user can act as a buyer and seller.
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
    user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT users_status_check CHECK (status IN ('active', 'suspended'))
);

CREATE TABLE seller_profiles (
    seller_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id UUID NOT NULL UNIQUE REFERENCES users (user_id),
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT seller_profiles_status_check CHECK (status IN ('active', 'suspended'))
);

CREATE TABLE stores (
    store_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    seller_id UUID NOT NULL REFERENCES seller_profiles (seller_id),
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT stores_name_not_blank CHECK (length(btrim(name)) > 0),
    CONSTRAINT stores_slug_not_blank CHECK (length(btrim(slug)) > 0)
);

CREATE TABLE categories (
    category_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_category_id UUID REFERENCES categories (category_id),
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT categories_name_not_blank CHECK (length(btrim(name)) > 0)
);

CREATE TABLE products (
    product_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    seller_id UUID NOT NULL REFERENCES seller_profiles (seller_id),
    category_id UUID REFERENCES categories (category_id),
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    product_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    price_minor BIGINT NOT NULL,
    currency CHAR(3) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT products_title_not_blank CHECK (length(btrim(title)) > 0),
    CONSTRAINT products_description_not_blank CHECK (length(btrim(description)) > 0),
    CONSTRAINT products_type_check CHECK (product_type IN ('digital', 'physical_standard')),
    CONSTRAINT products_status_check CHECK (
        status IN ('draft', 'pending_review', 'published', 'suspended', 'archived')
    ),
    CONSTRAINT products_price_non_negative CHECK (price_minor >= 0),
    CONSTRAINT products_currency_check CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE INDEX products_public_search_idx
    ON products (status, product_type, category_id, currency);

CREATE TABLE product_variants (
    product_variant_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products (product_id),
    sku TEXT NOT NULL,
    price_minor BIGINT NOT NULL,
    currency CHAR(3) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT product_variants_sku_not_blank CHECK (length(btrim(sku)) > 0),
    CONSTRAINT product_variants_price_non_negative CHECK (price_minor >= 0),
    CONSTRAINT product_variants_currency_check CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT product_variants_product_sku_unique UNIQUE (product_id, sku)
);

CREATE TABLE inventory_items (
    product_variant_id UUID PRIMARY KEY REFERENCES product_variants (product_variant_id),
    available_stock BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT inventory_items_stock_non_negative CHECK (available_stock >= 0)
);

CREATE TABLE inventory_reservations (
    reservation_id UUID PRIMARY KEY,
    product_variant_id UUID NOT NULL REFERENCES product_variants (product_variant_id),
    quantity BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    released_at TIMESTAMPTZ,
    CONSTRAINT inventory_reservations_quantity_positive CHECK (quantity > 0),
    CONSTRAINT inventory_reservations_status_check CHECK (status IN ('active', 'released'))
);

CREATE INDEX inventory_reservations_variant_status_idx
    ON inventory_reservations (product_variant_id, status);
