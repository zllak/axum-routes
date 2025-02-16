use proc_macro2::{Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::Ident;

// Utility method to find the proper name of the `axum_routes` crate
pub(crate) fn axum_routes_crate() -> TokenStream {
    let found = crate_name("axum-routes").expect("axum_routes must be in Cargo.toml");
    match found {
        FoundCrate::Itself => quote!(crate),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
    }
}

/// Proper name for axum, support renaming
pub(crate) fn axum_crate() -> TokenStream {
    let found = crate_name("axum").expect("axum must be in Cargo.toml");
    match found {
        FoundCrate::Itself => quote!(crate),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
    }
}

/// For the builder structure, create the proper function name
pub(crate) fn builder_fn_name(ident: &Ident) -> Ident {
    quote::format_ident!("with_customizer_{}", ident)
}

pub(crate) fn builder_struct_name(ident: &Ident) -> Ident {
    quote::format_ident!("__{}Builder", ident)
}
