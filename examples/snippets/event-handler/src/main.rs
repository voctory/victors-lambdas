//! Event-handler snippet for documentation.

use aws_lambda_powertools::prelude::{HttpError, Method, Request, Response, Router};

fn main() {
    let mut router = Router::new();

    router.add_response_middleware(|request, response| {
        response.with_header("x-routed-path", request.path())
    });

    router.get("/orders/{order_id}", |request| {
        let order_id = request
            .path_param("order_id")
            .expect("route captures order_id");

        Response::ok(format!(r#"{{"order_id":"{order_id}"}}"#))
            .with_header("content-type", "application/json")
    });

    router.add_routes([Method::Get, Method::Post], "/health", |_| {
        Response::ok("ok")
    });

    router.post_fallible("/orders", |request| {
        if request.body().is_empty() {
            Err(HttpError::bad_request("missing request body"))
        } else {
            Ok(Response::new(202).with_body("accepted"))
        }
    });

    let order_response = router.handle(Request::new(Method::Get, "/orders/order-123"));
    assert_eq!(order_response.status_code(), 200);
    assert_eq!(order_response.body(), br#"{"order_id":"order-123"}"#);
    assert_eq!(
        order_response.header("x-routed-path"),
        Some("/orders/order-123"),
    );

    let health_response = router.handle(Request::new(Method::Post, "/health"));
    assert_eq!(health_response.status_code(), 200);
    assert_eq!(health_response.body(), b"ok");

    let bad_request = router.handle(Request::new(Method::Post, "/orders"));
    assert_eq!(bad_request.status_code(), 400);
    assert_eq!(bad_request.body(), b"missing request body");

    println!(
        "handled order route with status {}",
        order_response.status_code()
    );
}
