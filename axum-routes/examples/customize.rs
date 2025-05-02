#![allow(dead_code)]

use axum_routes::*;

pub async fn get() -> &'static str {
    "home"
}

#[routes]
enum Router {
    #[get("/", handler = get, customize = home_customizer)]
    Home,
    #[nest("/nest")]
    Nested(Nested),
}

#[routes]
enum Nested {
    #[get("/", handler = get)]
    OtherHome,
}

fn main() {
    let _routes = axum_routes::router!(Router, home_customizer = |route| route);

    router!(Router,
        Router::Home => |route| route,
        Router::Nested(_) => |router| router,
        Router::Nested(Nested::OtherHome) => |route| route,
        Router::Nested(Nested::OtherNested(_)) => |router| router,
    );

    // Somehow transform this into a HashMap<name, BoxedFn...>
    // and do that for top level Router, then Router::Nested, then
    // Router::Nested(Nested::OtherNested) ...
}
