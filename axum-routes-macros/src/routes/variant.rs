//! Struct that will contain all data gathered by the parsing of the TokenStream.
//! This will be used to construct the macro codegen

use super::method::Method;
use super::route::{Route, RouteComponent};
use crate::punctuated_attrs::PunctuatedAttrs;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    Attribute, Ident, Path,
};

// ----------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct Variant {
    // Name of the variant
    pub(crate) variant: Ident,
    // Span for proper error
    pub(crate) span: Span,
    // The variant kind
    pub(crate) kind: VariantKind,
    // Other attributes on that variant
    pub(crate) other_attributes: Vec<Attribute>,
    // Conditional compilation
    pub(crate) conditional_compilation: Vec<Attribute>,
}

impl ToTokens for Variant {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let other_attrs = &self.other_attributes;
        let variant = &self.variant;
        let conditional = &self.conditional_compilation;

        let ts = match &self.kind {
            VariantKind::Nest { ident, .. } => {
                // this should be safe, as we made sure we had one and
                // only one field
                let ident = ident.as_ref().unwrap();
                quote::quote! {
                    #(#conditional)*
                    #(#other_attrs)*
                    #variant(#ident)
                }
            }
            VariantKind::Method { .. } => {
                quote::quote! {
                    #(#conditional)*
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

impl Variant {
    /// Used in the resolve_route to generate the route
    pub(crate) fn match_statement(&self) -> proc_macro2::TokenStream {
        let krate = crate::util::axum_routes_crate();
        let variant = &self.variant;
        let conditional = &self.conditional_compilation;

        match &self.kind {
            VariantKind::Nest { route, .. } => {
                let components = route_components_to_tokenstream(&route.components);

                // Here we don't need to check at the end if we have to much
                // params, cause it will be handled by the recursive resolve_route
                quote::quote! {
                    #(#conditional)*
                    Self::#variant(nested) => {
                        vec![
                            #(#components),*,
                            nested.resolve_route(Vec::from(params))?
                        ].join("")
                    }
                }
            }
            VariantKind::Method { route, .. } => {
                let components = route_components_to_tokenstream(&route.components);

                quote::quote! {
                    #(#conditional)*
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
pub(crate) enum VariantKind {
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
        handler: Path,
        // Closure to call to customize the method router
        customize: Option<Ident>,
    },
}

/// Parses the attributes (only the part inside the #[...])
impl Parse for VariantKind {
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
        let route: Route = content.parse()?;
        let mut attributes_list: Option<PunctuatedAttrs<Ident, Path>> = None;
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
                let Some(handler) = attributes_list.remove("handler") else {
                    return Err(syn::Error::new(
                        ident.span(),
                        "should have an \"handler\" attribute",
                    ));
                };
                // Pop the customize
                let customize = attributes_list
                    .remove("customize")
                    .and_then(|ident| ident.get_ident().cloned());
                // No more attributes expected
                if let Some((name, _)) = attributes_list.iter().next() {
                    return Err(syn::Error::new(
                        name.span(),
                        format!("unknown {name} attribute"),
                    ));
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
                    customize = attributes_list
                        .remove("customize")
                        .and_then(|ident| ident.get_ident().cloned());
                    // If we have more attributes, it's an error
                    if let Some((name, _)) = attributes_list.iter().next() {
                        return Err(syn::Error::new(
                            name.span(),
                            format!("unknown {name} attribute"),
                        ));
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
