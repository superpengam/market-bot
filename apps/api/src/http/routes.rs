use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::{
    app::AppState,
    http::handlers::{
        ai::{add_ai_cart_item, create_ai_order, get_ai_order, search_ai_products},
        cart::{add_item, create_cart, get_cart, remove_item, update_item},
        moderation::{create_report, resolve_report, review_product, suspend_product},
        order::{cancel_order, create_order, get_order, preview_checkout},
        payment_webhook::receive_payment_webhook,
        product::get_product,
        product_search::search_products,
        seller::{
            create_product as create_seller_product, create_store,
            patch_product as patch_seller_product,
        },
    },
};

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/products/search", get(search_products))
        .route("/api/v1/products/{product_id}", get(get_product))
        .route("/api/v1/seller/stores", post(create_store))
        .route("/api/v1/seller/products", post(create_seller_product))
        .route(
            "/api/v1/seller/products/{product_id}",
            patch(patch_seller_product),
        )
        .route("/api/v1/carts", post(create_cart))
        .route("/api/v1/carts/{cart_id}", get(get_cart))
        .route("/api/v1/carts/{cart_id}/items", post(add_item))
        .route(
            "/api/v1/carts/{cart_id}/items/{item_id}",
            patch(update_item).delete(remove_item),
        )
        .route("/api/v1/checkout/preview", post(preview_checkout))
        .route("/api/v1/orders", post(create_order))
        .route("/api/v1/orders/{order_id}", get(get_order))
        .route("/api/v1/orders/{order_id}/cancel", post(cancel_order))
        .route("/api/v1/ai/products/search", get(search_ai_products))
        .route("/api/v1/ai/carts/{cart_id}/items", post(add_ai_cart_item))
        .route("/api/v1/ai/orders", post(create_ai_order))
        .route("/api/v1/ai/orders/{order_id}", get(get_ai_order))
        .route("/api/v1/payments/webhooks", post(receive_payment_webhook))
        .route(
            "/api/v1/admin/products/{product_id}/reviews",
            post(review_product),
        )
        .route(
            "/api/v1/admin/products/{product_id}/suspend",
            post(suspend_product),
        )
        .route("/api/v1/reports", post(create_report))
        .route(
            "/api/v1/admin/reports/{report_id}/resolve",
            post(resolve_report),
        )
}
