use crate::punctuated_attrs::PunctuatedAttrs;
use proc_macro::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    Expr, Ident, Path, Result, Token,
};

// router!(path::to::Router)
// router!(path::to::Router, custom_method = |router| {
//      router.with_state(...)
// })
struct Router {
    router_path: Path,
    _unused: Option<Token![,]>,
    // PunctuatedAttrs keys are unique, so this enforces customizer methods
    // to be unique.
    customize: PunctuatedAttrs<Ident, Expr>,
}

impl Parse for Router {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            router_path: input.parse()?,
            _unused: input.parse()?,
            customize: input.parse()?,
        })
    }
}

pub(crate) fn try_expand(input: TokenStream) -> Result<TokenStream> {
    let krate = crate::util::axum_routes_crate();

    let rm = syn::parse::<Router>(input)?;
    let router_path = rm.router_path;

    let customizers = rm
        .customize
        .iter()
        .map(|(ident, expr)| {
            let name = ident.to_string();
            quote::quote! {
                (#name, Box::new(#expr))
            }
        })
        .collect::<Vec<_>>();

    Ok(quote::quote! {
        {
            let registry = #krate::__private::CustomizerRegistry::from([
                #(#customizers),*
            ]);
            <#router_path as #krate::__private::Router>::build(&registry)
        }
    }
    .into())
}
