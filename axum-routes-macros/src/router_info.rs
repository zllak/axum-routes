//! Struct that will contain all data gathered by the parsing of the TokenStream.
//! This will be used to construct the macro codegen

use crate::{Method, Route, RouteComponent};
use proc_macro2::Span;
use quote::ToTokens;
use std::collections::HashMap;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned as _,
    token::Comma,
    Attribute, Error, Ident, Meta, Path,
};

// ----------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct RouterVariant {
    // Name of the variant
    pub(crate) variant: Ident,
    // Span for proper error
    pub(crate) span: Span,
    // The variant kind
    pub(crate) kind: RouterVariantKind,
    // Other attributes on that variant
    pub(crate) other_attributes: Vec<Attribute>,
}

impl ToTokens for RouterVariant {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let other_attrs = &self.other_attributes;
        let variant = &self.variant;

        let ts = match &self.kind {
            RouterVariantKind::Nest { ident, .. } => {
                // this should be safe, as we made sure we had one and
                // only one field
                let ident = ident.as_ref().unwrap();
                quote::quote! {
                    #(#other_attrs)*
                    #variant(#ident)
                }
            }
            RouterVariantKind::Method { .. } => {
                quote::quote! {
                    #(#other_attrs)*
                    #variant
                }
            }
        };
        tokens.extend(ts)
    }
}

fn route_components_to_tokenstream(components: &[RouteComponent]) -> Vec<proc_macro2::TokenStream> {
    let krate = crate::util::axum_routes_crate();

    components
        .iter()
        .map(|component| {
            match component {
                RouteComponent::Path(path) => quote::quote! { #path.to_string() },
                RouteComponent::Parameter(_) => quote::quote! { params.pop_front().ok_or(#krate::__private::RouteResolverError::ParametersMismatch)? },
            }
        })
        .collect()
}

impl RouterVariant {
    /// Used in the resolve_route to generate the route
    pub(crate) fn match_statement(&self) -> proc_macro2::TokenStream {
        let krate = crate::util::axum_routes_crate();
        let variant = &self.variant;

        match &self.kind {
            RouterVariantKind::Nest { route, .. } => {
                let components = route_components_to_tokenstream(&route.components);

                // Here we don't need to check at the end if we have to much
                // params, cause it will be handled by the recursive resolve_route
                quote::quote! {
                    Self::#variant(nested) => {
                        vec![
                            #(#components),*,
                            nested.resolve_route(Vec::from(params))?
                        ].join("")
                    }
                }
            }
            RouterVariantKind::Method { route, .. } => {
                let components = route_components_to_tokenstream(&route.components);

                quote::quote! {
                    Self::#variant => {
                        let resolved = vec![
                            #(#components),*
                        ].join("");
                        if !params.is_empty() {
                            return Err(#krate::__private::RouteResolverError::ParametersMismatch);
                        }
                        resolved
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) enum RouterVariantKind {
    Nest {
        // Nested Router ident
        ident: Option<Path>,
        // The route
        route: Route,
        // Closure to call to customize the whole generated Router
        customize: Option<Ident>,
    },
    Method {
        // Method name
        method: Method,
        // The route with all components
        route: Route,
        // Handler to call for this route
        handler: Ident,
        // Closure to call to customize the method router
        customize: Option<Ident>,
    },
}

/// Parses the attributes (only the part inside the #[...])
impl Parse for RouterVariantKind {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Expect an ident (nest, get, ...)
        let ident: Ident = input.parse()?;

        enum Type {
            Method(Method),
            Nested,
        }

        let r#type = match ident.to_string().to_lowercase().as_ref() {
            "get" => Type::Method(Method::Get),
            "post" => Type::Method(Method::Post),
            "delete" => Type::Method(Method::Delete),
            "put" => Type::Method(Method::Put),
            "head" => Type::Method(Method::Head),
            "options" => Type::Method(Method::Options),
            "trace" => Type::Method(Method::Trace),
            "patch" => Type::Method(Method::Patch),
            "any" => Type::Method(Method::Any),
            "nest" => Type::Nested,
            _ => return Err(syn::Error::new(
                ident.span(),
                "attribute name must be get, post, delete, put, head, options, trace, path or any",
            )),
        };

        let content;
        let _ = parenthesized!(content in input);

        // Parse the attribute (route(, (key = value)+)?)
        let route: crate::Route = content.parse()?;
        let mut attributes_list: Option<RouterAttributeList> = None;
        if !content.is_empty() {
            let _: syn::Token![,] = content.parse()?;
            attributes_list = Some(content.parse()?);
        }

        Ok(match r#type {
            Type::Method(method) => {
                let Some(mut attributes_list) = attributes_list else {
                    return Err(syn::Error::new(input.span(), "missing attributes"));
                };
                // Pop the handler
                let Some((_, handler)) = attributes_list.remove("handler") else {
                    return Err(syn::Error::new(
                        input.span(),
                        "should have an \"handler\" attribute",
                    ));
                };
                // Pop the customize
                let customize = attributes_list.remove("customize").map(|(_, ident)| ident);
                // No more attributes expected
                if let Some((name, (span, _))) = attributes_list.iter().next() {
                    return Err(syn::Error::new(*span, format!("unknown {name} attribute")));
                }

                Self::Method {
                    method,
                    route,
                    handler,
                    customize,
                }
            }
            Type::Nested => {
                let mut customize = None;

                if let Some(mut attributes_list) = attributes_list {
                    // Pop the customize if any
                    customize = attributes_list.remove("customize").map(|(_, ident)| ident);
                    // If we have more attributes, it's an error
                    if let Some((name, (span, _))) = attributes_list.iter().next() {
                        return Err(syn::Error::new(*span, format!("unknown {name} attribute")));
                    }
                }
                Self::Nest {
                    ident: None,
                    route,
                    customize,
                }
            }
        })
    }
}

// ----------------------------------------------------------------------------

/// The attributes in a #[...]
/// ie: handler = some_handler, id = Ty
#[derive(Debug, Default)]
pub(crate) struct RouterAttributeList {
    inner: HashMap<String, (Span, Ident)>,
}

impl TryFrom<Vec<(Ident, Ident)>> for RouterAttributeList {
    type Error = Error;

    fn try_from(value: Vec<(Ident, Ident)>) -> Result<Self, Self::Error> {
        let mut inner = HashMap::with_capacity(value.len());

        for (key, value) in value.into_iter() {
            let key_string = key.to_string();
            if inner.contains_key(&key_string) {
                return Err(syn::Error::new(key.span(), "duplicate attribute"));
            }

            inner.insert(key_string, (key.span(), value));
        }

        Ok(Self { inner })
    }
}

impl Parse for RouterAttributeList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Punctuated::<Meta, Comma>::parse_terminated(input)?
            .into_iter()
            .map(|meta| match meta {
                Meta::Path(_) | Meta::List(_) => Err(syn::Error::new(meta.span(), "sdf")),
                Meta::NameValue(name_value) => {
                    // Make sure the Path is actually a single Ident
                    let key_ident = name_value.path.get_ident().cloned().ok_or(syn::Error::new(
                        name_value.path.span(),
                        "attribute key must be a single identifier",
                    ))?;

                    // Make sure the value is a single Ident (extracted from an
                    // expected Path)
                    let value_ident = if let syn::Expr::Path(ref path) = name_value.value {
                        // Same as above, make sure we have a single Ident
                        path.path.get_ident().cloned().ok_or(syn::Error::new(
                            name_value.value.span(),
                            "attribute value must be a single identifier",
                        ))
                    } else {
                        Err(syn::Error::new(
                            name_value.value.span(),
                            "value must be an identifier",
                        ))
                    }?;

                    Ok((key_ident, value_ident))
                }
            })
            .collect::<Result<Vec<(Ident, Ident)>, _>>()?
            .try_into()
    }
}

impl RouterAttributeList {
    pub(crate) fn iter(&self) -> std::collections::hash_map::Iter<'_, String, (Span, Ident)> {
        self.inner.iter()
    }

    pub(crate) fn remove(&mut self, k: &str) -> Option<(Span, Ident)> {
        self.inner.remove(k)
    }
}
