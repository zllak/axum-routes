use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::{parse_macro_input, punctuated::Punctuated, spanned::Spanned, ItemEnum, Meta};

mod route;
use route::{Route, RouteComponent};
mod router;
mod router_info;
use router_info::{RouterVariant, RouterVariantKind};

mod util;

// ----------------------------------------------------------------------------

#[derive(Debug)]
enum Method {
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

// ----------------------------------------------------------------------------

/// The main macro to create a `axum::Router` from an enum.
#[proc_macro_attribute]
pub fn routes(attr: TokenStream, item: TokenStream) -> TokenStream {
    let krate = crate::util::axum_routes_crate();
    let krate_axum = crate::util::axum_crate();
    let attributes =
        parse_macro_input!(attr with Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let num = parse_macro_input!(item as ItemEnum);

    // Do not accept generics
    if let Some(param) = num.generics.params.first() {
        return syn::Error::new(param.span(), "enum should not be generic")
            .into_compile_error()
            .into();
    }

    // Parse attribute of the root `routes` macro
    for meta in attributes.into_iter() {
        match meta {
            Meta::Path(_) | Meta::List(_) => {
                return syn::Error::new(
                    num.ident.span(),
                    "parameters must be in the format #[routes(param = value, param2 = value)]",
                )
                .into_compile_error()
                .into();
            }
            Meta::NameValue(_) => {
                // TODO(zllak): we could handle fallback, with_state, .. here
                // as a meta list.
            }
        }
    }

    // Browse enum variants
    let variants = num.variants.into_iter().map(|variant| {
        // Variant must have at least one attribute
        if variant.attrs.is_empty() {
            return Err(syn::Error::new(
                variant.span(),
                "variant should have at least one attribute (get, post, delete, put, head, options, trace, path, any)",
            ));
        }

        // Variant must not have a discriminant
        if variant.discriminant.is_some() {
            return Err(syn::Error::new(variant.ident.span(), "variant must not have a discriminant"));
        }

        // Collect the attributes (nested, method)
        // At least one
        if variant.attrs.is_empty() {
            return Err(syn::Error::new(variant.span(), "attribute missing"));
        }

        let (attr, other_attributes) = variant.attrs.iter().try_fold((None, Vec::new()), |(mut attr, mut other_attrs), current_attr| {
            // We only accept List
            match &current_attr.meta {
                Meta::List(_) => {
                    // FIXME: this will fail if the attribute is another meta
                    let parsed_attr = syn::parse2::<RouterVariantKind>(current_attr.meta.to_token_stream())?;
                    let _ = attr.insert(parsed_attr);
                }
                Meta::Path(_) | Meta::NameValue(_) => {
                    other_attrs.push(current_attr.clone())
                }
            };

            Ok::<_, syn::Error>((attr, other_attrs))
        })?;
        // used multiple times so to avoid moving variant, use the span that we
        // will copy
        let variant_span = variant.span();

        let mut attr = attr.ok_or(syn::Error::new(variant_span, "variant must have an attribute (get, post, put, head, options, delete, nest)"))?;

        // Accept exactly one field for nested routers
        if let RouterVariantKind::Nest { ref mut ident, .. } = attr {
            match variant.fields {
                syn::Fields::Named(named) => {
                    return Err(syn::Error::new(named.span(), "variant must have exactly one field"));
                }
                syn::Fields::Unnamed(unnamed) => {
                    // Only one field allowed
                    if unnamed.unnamed.len() > 1 {
                        return Err(syn::Error::new(unnamed.span(), "only one nested router allowed"));
                    }
                    *ident = unnamed.unnamed.first().and_then(|field|
                        match &field.ty {
                            syn::Type::Path(type_path) => Some(type_path.path.clone()),
                            _ => None,
                        }
                    );
                    if ident.is_none() {
                        return Err(syn::Error::new(unnamed.span(), "only paths are allowed as field type"));
                    }
                }
                syn::Fields::Unit => {
                    return Err(syn::Error::new(variant_span, "nested routers requires a field with the router"));
                }
            };
        }

        Ok(RouterVariant {
            variant: variant.ident.clone(),
            span: variant_span,
            kind: attr,
            other_attributes,
        })
    }).collect::<Result<Vec<RouterVariant>, syn::Error>>();
    let variants = match variants {
        Ok(variants) => variants,
        Err(err) => return err.into_compile_error().into(),
    };

    let vis = num.vis;
    let name = num.ident;

    // Error if we find duplicates, and fold into an HashMap in the end
    let variants = variants
        .into_iter()
        .try_fold(HashMap::new(), |mut acc, variant| {
            // Duplicate found
            if acc.contains_key(&variant.variant) {
                return Err(syn::Error::new(variant.span, "Duplicate enum variant"));
            }
            acc.insert(variant.variant.clone(), variant);

            Ok(acc)
        });
    let variants = match variants {
        Ok(variants) => variants,
        Err(err) => return err.into_compile_error().into(),
    };

    // Now we can generate the proper code
    let variants_ident = variants.values();
    let variants_resolve_route = variants
        .values()
        .map(|variant| variant.match_statement())
        .collect::<Vec<_>>();
    let variants_nested = variants
        .values()
        .filter_map(|variant| match &variant.kind {
            RouterVariantKind::Nest { ident, .. } => ident.clone(),
            RouterVariantKind::Method { .. } => None,
        })
        .collect::<Vec<_>>();

    // Now create the quotes to create the axum::Router
    let routes = variants
        .values()
        .map(|variant| match &variant.kind {
            RouterVariantKind::Nest {
                ident,
                route,
                customize,
                ..
            } => {
                if let Some(customize) = customize {
                    let customize = customize.to_string();
                    let name = name.to_string();
                    quote::quote! {
                        router = customize.get(#customize).expect(format!("Router {} requires a customizer named {}", #name, #customize).as_str()).customize_router(router.nest(#route, #ident::routes(customize)));
                    }
                } else {
                    quote::quote! {
                        router = router.nest(#route, #ident::routes(customize));
                    }
                }
            }
            RouterVariantKind::Method {
                method,
                route,
                handler,
                customize,
                ..
            } => {
                if let Some(customize) = customize {
                    let customize = customize.to_string();
                    let name = name.to_string();
                    quote::quote! {
                        router = router.route(#route, customize.get(#customize).expect(format!("Router {} requires a customizer named {}", #name, #customize).as_str()).customize_route(#krate_axum::routing::#method(#handler)));
                    }
                } else {
                    quote::quote! {
                        router = router.route(#route, #krate_axum::routing::#method(#handler));
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #vis enum #name {
            #(#variants_ident),*
        }

        #(
        const _: #krate::__private::AssertFieldIsRouter<#variants_nested> = #krate::__private::AssertFieldIsRouter {
            _field: ::core::marker::PhantomData,
        };
        )*

        #vis impl #krate::__private::Router for self::#name {
            fn routes(customize: &std::collections::HashMap<&'static str, #krate::__private::RouteCustomizer>) -> #krate_axum::Router {
                let mut router = #krate_axum::Router::new();

                #(#routes)*

                router
            }

            fn resolve_route(&self, params: Vec<String>) -> Result<String, #krate::__private::RouteResolverError> {
                let mut params: std::collections::VecDeque<String> = params.into();
                Ok(match self {
                    #(#variants_resolve_route)*
                })
            }
        }
    }
    .into()
}

// ----------------------------------------------------------------------------

/// Public macro to create the router
#[proc_macro]
pub fn router(input: TokenStream) -> TokenStream {
    router::inner_router(input)
}
