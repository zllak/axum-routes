#[derive(Debug)]
pub(crate) enum Method {
    Get,
    Post,
    Delete,
    Put,
    Head,
    Options,
    Trace,
    Patch,
    Any,
}

impl quote::ToTokens for Method {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(match self {
            Method::Get => quote::quote! { get },
            Method::Post => quote::quote! { post },
            Method::Delete => quote::quote! { delete },
            Method::Put => quote::quote! { put },
            Method::Head => quote::quote! { head },
            Method::Options => quote::quote! { options },
            Method::Trace => quote::quote! { trace },
            Method::Patch => quote::quote! { patch },
            Method::Any => quote::quote! { any },
        })
    }
}
