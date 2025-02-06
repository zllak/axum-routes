#![allow(dead_code)]

use axum_routes::*;

pub async fn get() -> &'static str {
    "this is a handler"
}

pub async fn single_path() -> &'static str {
    "another handler"
}

#[routes]
enum Router {
    #[cfg(target_os = "linux")]
    #[get("/", handler = get)]
    Home,
    #[cfg(target_os = "macos")]
    #[get("/other", handler = single_path)]
    Other,
}
fn main() {
    let _routes = axum_routes::router!(Router);
}
