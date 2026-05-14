# Event Handler

The event-handler utility provides dependency-free HTTP request/response routing and optional adapters for Lambda event
sources. It is exposed through the `event-handler` Cargo feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["event-handler"] }
```

## Supported Behavior

- `Router` and `AsyncRouter` for sync and async route dispatch.
- `Request`, `Response`, `Method`, and `PathParams` types independent of a specific Lambda event source.
- Static and dynamic route matching with static path precedence and exact-method precedence over `ANY`.
- Multi-method route registration, custom not-found handlers, and router composition with path prefixes.
- Fallible route handlers with built-in HTTP errors, catch-all error handlers, and typed error handlers.
- Router-level and route-specific request/response middleware.
- Request-scoped typed extensions for middleware-to-handler data, and router shared typed extensions for values reused
  across requests.
- Matched-route metadata on routed requests so handlers and response middleware can label observations by route pattern.
- Optional CORS preflight handling and response headers with request `Origin` matching, wildcard/additional origins,
  AWS-friendly default request headers, and credential headers for non-wildcard origins.
- Optional metrics middleware through `event-handler-metrics` for per-route latency, fault, and error metrics.
- Optional trace record middleware through `event-handler-tracer` for exporter-neutral per-route `TraceSegment` records.
- Optional AppSync GraphQL scalar helpers for `ID`, `AWSDate`, `AWSTime`, `AWSDateTime`, and `AWSTimestamp`.
- Optional request/response validation hooks through `event-handler-validation`.
- Optional gzip and deflate response compression through `event-handler-compression`.
- Optional adapters for API Gateway REST API, HTTP API, and WebSocket API events, Application Load Balancer events,
  Lambda Function URL events, VPC Lattice v1/v2 events, AppSync direct and batch resolvers, AppSync Events, Bedrock
  Agent OpenAPI action groups, and Bedrock Agent function-details action groups.

## Event Source Adapters

Enable `event-handler-aws-lambda-events` to convert common `aws_lambda_events` HTTP event models to and from the
dependency-free `Request` and `Response` types:

```toml
aws-lambda-powertools = { version = "0.1", features = ["event-handler-aws-lambda-events"] }
```

AppSync scalar helpers, AppSync Events, and Bedrock Agent function-details resolvers have separate feature flags so
applications can opt into only the helpers and adapters they use:

```toml
aws-lambda-powertools = { version = "0.1", features = [
  "event-handler-appsync-scalars",
  "event-handler-appsync-events",
  "event-handler-bedrock-agent-functions",
] }
```

Metrics middleware is available when both the event handler and metrics utility are enabled:

```toml
aws-lambda-powertools = { version = "0.1", features = ["event-handler-metrics"] }
```

Trace record middleware is available when both the event handler and tracer utility are enabled:

```toml
aws-lambda-powertools = { version = "0.1", features = ["event-handler-tracer"] }
```

## Snippet

The buildable snippet in [examples/snippets/event-handler/src/main.rs](../../examples/snippets/event-handler/src/main.rs)
registers dynamic and multi-method routes, shares typed extension data between middleware and handlers, adds response
middleware, and maps a fallible route error to an HTTP response.

Run it locally with:

```sh
cargo run -p event-handler-snippet
```
