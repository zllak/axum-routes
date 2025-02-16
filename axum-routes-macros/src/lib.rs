mod router;
mod routes;

mod punctuated_attrs;
mod util;

// ----------------------------------------------------------------------------

use proc_macro::TokenStream;

/// The main macro to create an [`axum::Router`](https://docs.rs/axum/latest/axum/struct.Router.html)
/// from an enum.
#[proc_macro_attribute]
pub fn routes(attr: TokenStream, item: TokenStream) -> TokenStream {
    match routes::try_expand(attr, item) {
        Ok(expanded) => expanded,
        Err(err) => err.into_compile_error().into(),
    }
}

// ----------------------------------------------------------------------------

/// Create the [`axum::Router`](https://docs.rs/axum/latest/axum/struct.Router.html) instance.
/// The first parameter of the macro must be the path to a type that was created
/// from the [`macro@routes`] macro.
#[proc_macro]
pub fn router(input: TokenStream) -> TokenStream {
    match router::try_expand(input) {
        Ok(expanded) => expanded,
        Err(err) => err.into_compile_error().into(),
    }
}
