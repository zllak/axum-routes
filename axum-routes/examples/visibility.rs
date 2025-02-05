use axum_routes::*;

pub mod router {
    use super::*;

    #[routes]
    pub enum Router {
        #[get("/", handler = self::other::get)]
        Home,
    }

    mod other {
        pub async fn get() -> &'static str {
            "handler"
        }
    }
}

fn main() {
    let _router = axum_routes::router!(router::Router);
}
