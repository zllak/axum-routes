#![allow(dead_code)]

use axum_routes::*;

mod mymod {
    pub async fn get() -> &'static str {
        "this is a handler"
    }
}

pub async fn single_path() -> &'static str {
    "another handler"
}

#[routes]
enum Router {
    #[get("/", handler = mymod::get)]
    Home,
    #[get("/other", handler = single_path)]
    Other,
}
fn main() {
    let _routes = axum_routes::router!(Router);
}
