use axum::{extract::Request, middleware::Next, response::Response};
use tower_http::request_id::RequestId;

pub async fn request_id_middleware(request: Request, next: Next) -> Response {
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map(|id| {
            id.header_value()
                .to_str()
                .unwrap_or("unknown")
                .to_string()
        })
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let span = tracing::info_span!("request", id = %request_id);
    let _guard = span.entered();

    let mut response = next.run(request).await;

    response
        .headers_mut()
        .insert("X-Request-Id", request_id.parse().unwrap());

    response
}
