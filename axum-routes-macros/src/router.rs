use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned as _,
    Expr, Ident, Path, Result, Token,
};

enum RouteCustomizerMethod {
    Router(Expr),
    MethodRouter(Expr),
}
impl Parse for RouteCustomizerMethod {
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
impl ToTokens for RouteCustomizerMethod {
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

struct MetaListIdentPath {
    ident: Ident,
    #[allow(dead_code)]
    eq_token: Token![=],
    path: RouteCustomizerMethod,
}

impl Parse for MetaListIdentPath {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            eq_token: input.parse()?,
            path: input.parse()?,
        })
    }
}

struct RouterMacro {
    router_path: Path,
    comma: Option<Token![,]>,
    customize: Option<Punctuated<MetaListIdentPath, Token![,]>>,
}

impl Parse for RouterMacro {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            router_path: input.parse()?,
            comma: input.parse()?,
            customize: Some(Punctuated::<MetaListIdentPath, Token![,]>::parse_terminated(input)?),
        })
    }
}

pub(crate) fn inner_router(input: TokenStream) -> TokenStream {
    let krate = crate::util::axum_routes_crate();

    // Expect the Path of the enum implementing Router
    let rm = parse_macro_input!(input as RouterMacro);
    let router_path = rm.router_path;

    if rm.comma.is_none() && rm.customize.is_none() {
        quote::quote! {
            <#router_path as #krate::__private::Router>::routes(&::std::collections::HashMap::new())
        }
        .into()
    } else if let Some(punctuated) = rm.customize {
        let args = punctuated
            .into_iter()
            .map(|item| {
                //
                let ident = item.ident.to_string();
                let path = item.path;
                quote::quote! {
                    (#ident, #krate::__private::RouteCustomizer::#path)
                }
            })
            .collect::<Vec<_>>();
        quote::quote! {
            {
                <#router_path as #krate::__private::Router>::routes(&::std::collections::HashMap::from([
                    #(#args),*
                ]))
            }
        }
        .into()
    } else {
        syn::Error::new(rm.comma.span(), "missing arguments after comma")
            .into_compile_error()
            .into()
    }
}
