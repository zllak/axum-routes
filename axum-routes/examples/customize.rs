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
}
