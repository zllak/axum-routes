#![allow(dead_code)]

use axum::{Json, extract::Path, http::StatusCode, response::IntoResponse};
use axum_routes::*;
use serde_json::json;

pub async fn get() -> &'static str {
    "home"
}

pub async fn get_user_by_id(Path(_user_id): Path<u32>) -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"username": "Name"})))
}

pub async fn get_field_by_id(
    Path(user_id): Path<u32>,
    Path(field): Path<String>,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({"id": user_id, "username": "Name", "field": field})),
    )
}

#[routes]
enum Router {
    #[get("/", handler = get, customize = home_customizer)]
    Home,
    #[nest("/users")]
    Users(Users),
}

#[routes]
enum Users {
    #[get("/{id}", handler = get_user_by_id)]
    GetByID,
    #[get("/{id}/field/{field}", handler = get_field_by_id)]
    GetFieldByID,
}

fn main() {
    let _routes = axum_routes::router!(Router, home_customizer = $|route| route);

    // Resolve allows us to recreate the route
    assert_eq!(
        "/users/32",
        axum_routes::resolve!(Router::Users(Users::GetByID), 32).unwrap()
    );
    assert_eq!("/32", axum_routes::resolve!(Users::GetByID, 32).unwrap());
    assert_eq!(
        "/users/42/field/address",
        axum_routes::resolve!(Router::Users(Users::GetFieldByID), 42, "address").unwrap()
    );
}
