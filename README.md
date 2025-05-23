# axum-routes

`axum-routes` is a crate on top of [axum](https://github.com/tokio-rs/axum) to
declare routers through enums, and resolve easily routes, so we don't have to
hardcode routes when linking in web apps.

[![Crates.io](https://img.shields.io/crates/v/axum-routes)](https://crates.io/crates/axum-routes)
[![Documentation](https://docs.rs/axum-routes/badge.svg)](https://docs.rs/axum-routes)

## Features
- Declare your `axum::Router` using enums
- Customize routes/nested routers (layers, with_state, fallback, ...)
- Resolve links using the enum, removing the need to hardcode

## Example

```rust
use axum::{extract::Path, Json};
use axum_routes::routes;
use http::StatusCode;
use serde_json::{json, Value};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

#[routes]
enum MyApp {
    #[nest("/api", customize = custom_api)]
    Api(APIRoutes),
    #[get("/", handler = home)]
    Home,
    #[get("/about", handler = about)]
    About,
}

#[routes]
enum APIRoutes {
    #[get("/users/:id", handler = get_users_by_id)]
    GetUsersByID,
    #[post("/users", handler = create_user)]
    CreateUser,
}

// ----------------------------------------------------------------------------
// Handlers

async fn home() -> &'static str {
    "Hello world!"
}
async fn about() {}
async fn get_users_by_id(Path(_user_id): Path<u32>) {}
async fn create_user(Json(_payload): Json<Value>) -> (StatusCode, Json<Value>) {
    (StatusCode::CREATED, Json(json!({"status": "ok"})))
}

// ----------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let trace_layer = ServiceBuilder::new().layer(TraceLayer::new_for_http());
    let app = axum_routes::router!(MyApp,
        custom_api = #move |route| {
            route.layer(trace_layer.clone())
        },
    );

    // Resolve parameterized routes, removing the need to hardcode routes
    assert_eq!(
        "/api/users/42",
        axum_routes::resolve!(MyApp::Api(APIRoutes::GetUsersByID), 42).unwrap()
    );

    // run the app
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Contributing

See a bug ? An improvement ? A new feature you want ? Feel free to open an issue,
or even a PR.

This project is not affiliated with `axum` at all.
