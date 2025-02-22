mod method;
mod route;
mod variant;

use self::variant::{Variant, VariantKind};
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::{punctuated::Punctuated, spanned::Spanned, ItemEnum, Meta, Result};

pub fn try_expand(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let krate = crate::util::axum_routes_crate();
    let krate_axum = crate::util::axum_crate();

    let attributes =
        syn::parse::Parser::parse(<Punctuated<Meta, syn::Token![,]>>::parse_terminated, attr)?;
    let num = syn::parse::<ItemEnum>(item)?;

    // Do not accept generics
    if let Some(param) = num.generics.params.first() {
        return Err(syn::Error::new(param.span(), "enum should not be generic"));
    }

    // Parse attribute of the root `routes` macro
    for meta in attributes {
        match meta {
            Meta::Path(_) | Meta::List(_) => {
                return Err(syn::Error::new(
                    num.ident.span(),
                    "parameters must be in the format #[routes(param = value, param2 = value)]",
                ));
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

        let (attr, other_attributes, conditional) = variant.attrs.iter().try_fold((None, Vec::new(), Vec::new()), |(mut attr, mut other_attrs, mut conditional), current_attr| {
            // We only accept List
            match &current_attr.meta {
                Meta::List(list) => {
                    // We must check a little bit the list to determine early
                    // if the attribute is one of ours or not.
                    let attr_ident = list.path.segments.first().map(|segment| segment.ident.to_string());
                    if let Some(attr_ident) = attr_ident {
                        match attr_ident.as_ref() {
                            "cfg" | "cfg_attr" => {
                                // Conditional compilation
                                conditional.push(current_attr.clone());
                            }
                            "get" | "post" | "put" | "head" | "options" | "delete" | "any" | "nest" => {
                                let parsed_attr = syn::parse2::<VariantKind>(current_attr.meta.to_token_stream())?;
                                let _ = attr.insert(parsed_attr);
                            }
                            _ => {
                                // Entirely another attribute that we don't know
                                other_attrs.push(current_attr.clone());
                            }
                        }
                    }
                }
                Meta::Path(_) | Meta::NameValue(_) => {
                    other_attrs.push(current_attr.clone())
                }
            };

            Ok::<_, syn::Error>((attr, other_attrs, conditional))
        })?;
        // used multiple times so to avoid moving variant, use the span that we
        // will copy
        let variant_span = variant.span();

        let mut attr = attr.ok_or(syn::Error::new(variant_span, "variant must have an attribute (get, post, put, head, options, delete, any, nest)"))?;

        // Accept exactly one field for nested routers
        if let VariantKind::Nest { ref mut ident, .. } = attr {
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

        Ok(Variant {
            variant: variant.ident.clone(),
            span: variant_span,
            kind: attr,
            other_attributes,
            conditional_compilation: conditional,
        })
    }).collect::<Result<Vec<Variant>>>()?;

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
        })?;

    // Now we can generate the proper code
    let variants_ident = variants.values();
    let variants_resolve_route = variants
        .values()
        .map(|variant| variant.match_statement())
        .collect::<Vec<_>>();
    let variants_nested = variants
        .values()
        .filter_map(|variant| match &variant.kind {
            VariantKind::Nest { ident, .. } => ident.clone(),
            VariantKind::Method { .. } => None,
        })
        .collect::<Vec<_>>();

    // Now create the quotes to create the axum::Router
    let routes = variants
        .values()
        .map(|variant| {
            match &variant.kind {
                VariantKind::Nest {
                    ident,
                    route,
                    customize,
                    ..
                } => {
                    let conditional = &variant.conditional_compilation;

                    if let Some(customize) = customize {
                        let customize_name = customize.to_string();
                        quote::quote! {
                            #(#conditional)*
                            {
                                router = router.nest(
                                    #route,
                                    (registry.require::<#krate::__private::BoxedFnRouterCustomizer>(#customize_name))(
                                        <#ident as #krate::__private::Router>::build(registry)
                                    ),
                                );
                            }
                        }
                    } else {
                        quote::quote! {
                            #(#conditional)*
                            {
                                router = router.nest(#route, <#ident as #krate::__private::Router>::build(registry));
                            }
                        }
                    }
                }
                VariantKind::Method {
                    method,
                    route,
                    handler,
                    customize,
                    ..
                } => {
                    let conditional = &variant.conditional_compilation;

                    if let Some(customize) = customize {
                        let customize_name = customize.to_string();
                        quote::quote! {
                            #(#conditional)*
                            {
                                router = router.route(
                                    #route,
                                    (registry.require::<#krate::__private::BoxedFnMethodCustomizer>(#customize_name))(
                                        #krate_axum::routing::#method(#handler)
                                    ),
                                );
                            }
                        }
                    } else {
                        quote::quote! {
                            #(#conditional)*
                            {
                                router = router.route(
                                    #route,
                                    #krate_axum::routing::#method(
                                        #handler
                                    )
                                );
                            }
                        }
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    // TODO(zllak): for the builder fields & methods, we use the customizer name
    // Maybe it would be better to completely ignore this given name and generate
    // our own, to avoid any possible weird character in the name.
    // This might not be a problem as it must be parsed by syn so it must be
    // a valid Rust ident.

    Ok(quote! {
        #vis enum #name {
            #(#variants_ident),*
        }

        #(
        const _: #krate::__private::AssertFieldIsRouter<#variants_nested> = #krate::__private::AssertFieldIsRouter {
            _field: ::core::marker::PhantomData,
        };
        )*

        impl #krate::__private::Router for self::#name {
            #[allow(unused_variables, reason = "might be unused if no customizers")]
            fn build(registry: &#krate::__private::CustomizerRegistry) -> #krate_axum::Router {
                let mut router = #krate_axum::Router::default();
                #(#routes)*
                router
            }

            fn resolve_route(&self, params: Vec<String>) -> Result<String, #krate::__private::RouteResolverError> {
                let params: std::collections::VecDeque<String> = params.into();
                Ok(match self {
                    #(#variants_resolve_route)*
                })
            }
        }
    }
    .into())
}
