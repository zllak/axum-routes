//! # Axum Routes
//!
//! Create an [`axum::Router`](https://docs.rs/axum/latest/axum/struct.Router.html) from an enum.
//! You can then use this enum to resolve the routes and avoid hardcoding
//! routes in your project.
//!
//! ```ignore,rust
//! #[routes]
//! enum RoutesUsers {
//!     #[post("/", handler = create_user)]
//!     CreateUser,
//!     #[get("/:id", handler = get_user)]
//!     GetByID,
//!     #[put("/:id", handler = edit_user)]
//!     EditUser,
//!     #[delete("/:id", handler = delete_user)]
//!     DeleteByID,
//!     #[get("/other/:id", handler = get_other_resource)]
//!     GetOtherResourceByID,
//! }
//!
//! #[routes]
//! enum Routes {
//!     #[nest("/users")]
//!     Users(RoutesUsers),
//!     #[get("/", handler = main)]
//!     Main,
//! }
//!
//! async fn create_user() {} // axum handler
//! async fn get_user() {}
//! async fn edit_user() {}
//! async fn delete_user() {}
//! async fn get_other_resource() {}
//! ```
//!
//! The route path (ie "/:id") is exactly what `axum` supports (underneath
//! it uses the `matchit` crate. At the moment of writing, `axum` still uses
//! `matchit 0.7.x`, so the parameters are declare `:param`. In `matchit 0.8.x`,
//! parameters now are declared `{param}`.
//! So to ease the process, `axum-routes` supports both.
//!
//! ## Resolving routes
#![doc = include_str!("../RESOLVE.md")]
//! ## Create the [`axum::Router`]
#![doc = include_str!("../ROUTER.md")]

/// Resolve the given router enum variant with the given parameter.
/// This should not be called directly, let the macros call it.
#[doc(hidden)]
pub fn route_resolver<R: crate::__private::Router, T>(
    router: R,
    params: impl crate::__private::RouteParameters<T>,
) -> Result<String, crate::__private::RouteResolverError> {
    router.resolve_route(params.parameters())
}

#[doc(inline)]
pub use axum_routes_macros::{router, routes};

/// Resolve a route.
///
#[doc = include_str!("../RESOLVE.md")]
// FIXME(zllak): this might not handle a mix of ident and literals
#[macro_export]
macro_rules! resolve {
    ($route:expr) => {
        $crate::route_resolver($route, ())
    };
    ($route:expr, $($pp:ident),+) => {
        $crate::route_resolver($route, ($($pp,)*))
    };
    ($route:expr, $($pp:literal),+) => {
        $crate::route_resolver($route, ($($pp,)*))
    };
}

// ----------------------------------------------------------------------------

#[doc(hidden)]
pub mod __private {
    /// This should never be used directly, only here to ensure
    /// nested fields implement the Router trait
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct AssertFieldIsRouter<T: crate::__private::Router + ?Sized> {
        pub _field: core::marker::PhantomData<T>,
    }

    /// For customizers, as we are nesting routers that can also use
    /// customizers, but there is no way of knowing which nested router
    /// uses which customizers, we need to have an hashmap of customizers.
    /// And as we are supporting different customizers, we need some kind of
    /// type erasure.
    #[doc(hidden)]
    pub struct CustomizerRegistry {
        inner: ::std::collections::HashMap<&'static str, Box<dyn ::std::any::Any>>,
    }

    impl CustomizerRegistry {
        pub fn get<T: 'static>(&self, name: &'static str) -> Option<&T> {
            self.inner
                .get(name)
                .and_then(|boxed| boxed.downcast_ref::<T>())
        }

        #[expect(clippy::panic, reason = "expected to panic if missing")]
        pub fn require<T: 'static>(&self, name: &'static str) -> &T {
            self.get::<T>(name)
                .unwrap_or_else(|| panic!("missing customizer with name: {name}"))
        }
    }

    impl<V, const N: usize> From<[(&'static str, V); N]> for CustomizerRegistry
    where
        V: ::std::any::Any,
    {
        fn from(value: [(&'static str, V); N]) -> Self {
            Self {
                inner: ::std::collections::HashMap::from_iter(
                    value
                        .into_iter()
                        .map(|(k, v)| (k, Box::new(v) as Box<dyn ::std::any::Any>)),
                ),
            }
        }
    }

    #[doc(hidden)]
    #[derive(thiserror::Error, Debug)]
    pub enum RouteResolverError {
        #[error("parameter mismatch")]
        ParametersMismatch,
    }

    #[doc(hidden)]
    pub type BoxedFnMethodCustomizer =
        Box<dyn Fn(axum::routing::MethodRouter) -> axum::routing::MethodRouter>;
    #[doc(hidden)]
    pub type BoxedFnRouterCustomizer = Box<dyn Fn(axum::Router) -> axum::Router>;

    /// Trait to generate the routes for an enum
    #[doc(hidden)]
    pub trait Router {
        /// Generates the router
        fn build(registry: &crate::__private::CustomizerRegistry) -> axum::Router;
        /// Resolve the given enum variant with the parameters
        fn resolve_route(
            &self,
            params: Vec<String>,
        ) -> Result<String, crate::__private::RouteResolverError>;
    }

    /// This trait is here to handle an unknown number of parameters
    /// Only constraint is that each parameter implements ToString
    #[doc(hidden)]
    pub trait RouteParameters<T>: Sized {
        fn parameters(&self) -> Vec<String>;
    }

    // Support empty parameters
    impl RouteParameters<()> for () {
        fn parameters(&self) -> Vec<String> {
            Vec::new()
        }
    }

    macro_rules! impl_route_parameters {
    ($($ty:ident),+) => {

        #[allow(non_snake_case)]
        impl<$($ty,)*> $crate::__private::RouteParameters<($($ty,)*)> for ($($ty,)*)
        where
            $( $ty: ToString, )*
        {
            fn parameters(&self) -> Vec<String> {
                let ($($ty,)*) = self;
                Vec::from([
                    $( $ty.to_string(), )*
                ])
            }
        }

    };
}

    impl_route_parameters!(A);
    impl_route_parameters!(A, B);
    impl_route_parameters!(A, B, C);
    impl_route_parameters!(A, B, C, D);
    impl_route_parameters!(A, B, C, D, E);
    impl_route_parameters!(A, B, C, D, E, F);
    impl_route_parameters!(A, B, C, D, E, F, G);
    impl_route_parameters!(A, B, C, D, E, F, G, H);
    impl_route_parameters!(A, B, C, D, E, F, G, H, I);
    impl_route_parameters!(A, B, C, D, E, F, G, H, I, J);
}
