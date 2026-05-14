//! Event-handler snippet for documentation.

use victors_lambdas::prelude::{HttpError, Method, Request, Response, Router};

#[derive(Debug)]
struct RequestId(String);

#[derive(Debug)]
struct ServiceName(&'static str);

fn main() {
    let mut router = Router::new().with_shared_extension(ServiceName("checkout"));

    router.add_request_middleware(|request| {
        request.with_extension(RequestId("request-123".to_owned()))
    });

    router.add_response_middleware(|request, response| {
        let request_id = request
            .extension::<RequestId>()
            .map_or("missing", |request_id| request_id.0.as_str());
        response
            .with_header("x-routed-path", request.path())
            .with_header("x-request-id", request_id)
    });

    router.get("/orders/{order_id}", |request| {
        let order_id = request
            .path_param("order_id")
            .expect("route captures order_id");
        let service_name = request
            .shared_extension::<ServiceName>()
            .map_or("unknown", |service_name| service_name.0);

        Response::ok(format!(
            r#"{{"order_id":"{order_id}","service":"{service_name}"}}"#
        ))
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
    assert_eq!(
        order_response.body(),
        br#"{"order_id":"order-123","service":"checkout"}"#
    );
    assert_eq!(
        order_response.header("x-routed-path"),
        Some("/orders/order-123"),
    );
    assert_eq!(order_response.header("x-request-id"), Some("request-123"));

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
