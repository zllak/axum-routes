use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, Path, Result, Token,
};

enum CustomizerMethod {
    Router(Expr),
    MethodRouter(Expr),
}

impl Parse for CustomizerMethod {
    fn parse(input: ParseStream) -> Result<Self> {
        // #path or $path
        if input.peek(Token![#]) {
            let _: Token![#] = input.parse()?;
            Ok(Self::Router(input.parse()?))
        } else if input.peek(Token![$]) {
            let _: Token![$] = input.parse()?;
            Ok(Self::MethodRouter(input.parse()?))
        } else {
            Err(syn::Error::new(
                input.span(),
                "expected either #method or $method",
            ))
        }
    }
}

impl ToTokens for CustomizerMethod {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let q = match self {
            Self::Router(path) => quote::quote! {
                Router(Box::new(#path))
            },
            Self::MethodRouter(path) => quote::quote! {
                MethodRouter(Box::new(#path))
            },
        };
        tokens.extend(q);
    }
}

// Similar to syn::MetaNameValue, in the form ident = <customizer>
struct MetaName {
    ident: Ident,
    _unused: Token![=],
    path: CustomizerMethod,
}

impl Parse for MetaName {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            _unused: input.parse()?,
            path: input.parse()?,
        })
    }
}

// router!(path::to::Router)
// router!(path::to::Router, custom_method = #|router| {
//      router.with_state(...)
// })
struct Router {
    router_path: Path,
    _unused: Option<Token![,]>,
    customize: Option<Punctuated<MetaName, Token![,]>>,
}

impl Parse for Router {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            router_path: input.parse()?,
            _unused: input.parse()?,
            customize: Some(Punctuated::<MetaName, Token![,]>::parse_terminated(input)?),
        })
    }
}

// TODO zllak:
// New idea for this, remove # and $.
// Instead, change the #[routes] to make the router type implement
// methods that we could be calling here.

pub(crate) fn try_expand(input: TokenStream) -> Result<TokenStream> {
    let krate = crate::util::axum_routes_crate();

    // Expect the Path of the enum implementing Router
    let rm = syn::parse::<Router>(input)?;
    let router_path = rm.router_path;

    let args = rm
        .customize
        .unwrap_or_default()
        .into_iter()
        .map(|MetaName { ident, path, .. }| {
            // Important to call to_string here otherwise it will expand to
            // the ident and not a quoted string.
            let ident = ident.to_string();
            quote::quote! {
                (#ident, #krate::__private::RouteCustomizer::#path)
            }
        })
        .collect::<Vec<_>>();

    Ok(quote::quote! {
        {
            <#router_path as #krate::__private::Router>::routes(&::std::collections::HashMap::from([
                #(#args),*
            ]))
        }
    }
    .into())
}
