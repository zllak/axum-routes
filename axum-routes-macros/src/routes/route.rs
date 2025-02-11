use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    LitStr,
};

/// The HTTP route as a string
/// ie /this/:param/:other
#[derive(Debug, Default)]
pub(crate) struct Route {
    /// The route as a string
    pub(crate) route: String,
    /// The parameters found in this route
    pub(crate) components: Vec<RouteComponent>,
}

impl TryFrom<String> for Route {
    type Error = ParseRouteError;

    fn try_from(route: String) -> Result<Self, Self::Error> {
        let components = parse_route_into_components(&route)?;

        Ok(Route { route, components })
    }
}

impl Parse for Route {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let route: LitStr = input.parse()?;
        let value = route.value();

        if !value.starts_with("/") {
            return Err(syn::Error::new(input.span(), "route must start with /"));
        }

        value
            .try_into()
            .map_err(|_| syn::Error::new(route.span(), "invalid route"))
    }
}

impl ToTokens for Route {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let route = &self.route;
        tokens.extend(quote::quote! { #route });
    }
}

// ----------------------------------------------------------------------------

use monch::*;

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
#[error("Invalid route")]
pub struct ParseRouteError {
    #[source]
    pub(crate) source: ParseErrorFailureError,
}

impl ParseRouteError {
    #[allow(dead_code)]
    pub fn message(&self) -> &str {
        &self.source.message
    }
}

/// Parses a route (string), and return a list containing the components
/// of the route, meaning either a path, or a parameter
/// We are now depending on axum 0.8, so old version of matchit will produce
/// a panic to avoid silent fails. So we only support new matchit pattern.
///
/// Grammar:
///
/// route        ::= ( path | parameter )+
/// path         ::= [^{:]*
/// parameter    ::= ( '{' [^}]+ '}' )
pub fn parse_route_into_components(input: &str) -> Result<Vec<RouteComponent>, ParseRouteError> {
    with_failure_handling(|input| many1(or(path, parameter))(input))(input)
        .map_err(|err| ParseRouteError { source: err })
}

fn path(input: &str) -> ParseResult<RouteComponent> {
    map(if_not_empty(take_while(|c| c != '{' && c != ':')), |text| {
        RouteComponent::Path(text.to_owned())
    })(input)
}

fn parameter(input: &str) -> ParseResult<RouteComponent> {
    terminated(
        preceded(
            ch('{'),
            map(if_not_empty(take_while(|c| c != '}')), |text| {
                RouteComponent::Parameter(text.to_owned())
            }),
        ),
        ch('}'),
    )(input)
}

/// A component of a route, either a path or a parameter
/// This will be used to reconstruct the route with the
/// parameter injected properly
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteComponent {
    // A Path, meaning a part of the route (ie /users/)
    Path(String),
    // A Parameter, with it's name
    Parameter(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_matchit() -> Result<(), ParseRouteError> {
        let ret = parse_route_into_components("/company/:company_id/users/:user_id/list");
        let expected_err = Err(ParseRouteError {
            source: monch::ParseErrorFailureError {
                message: String::from("Unexpected character."),
                code_snippet: Some(String::from(":company_id/users/:user_id/list")),
            },
        });
        assert_eq!(ret, expected_err);

        Ok(())
    }

    #[test]
    fn parsing() -> Result<(), ParseRouteError> {
        // TODO(zllak): some special fuzzying could be interesting
        assert_eq!(
            parse_route_into_components("/segment{param}/")?,
            Vec::from([
                RouteComponent::Path("/segment".into()),
                RouteComponent::Parameter("param".into()),
                RouteComponent::Path("/".into()),
            ])
        );
        assert_eq!(
            parse_route_into_components("/{&*(}/segment?wer")?,
            Vec::from([
                RouteComponent::Path("/".into()),
                RouteComponent::Parameter("&*(".into()),
                RouteComponent::Path("/segment?wer".into()),
            ])
        );

        Ok(())
    }
}
