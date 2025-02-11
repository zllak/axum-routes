use proc_macro2::{Ident, Span};
use std::{collections::HashMap, hash::Hash};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    Error, Meta, MetaNameValue, Path, PathSegment,
};

/// The attributes in a #[...]
/// ie: handler = some_handler, id = Ty
#[derive(Debug)]
pub(crate) struct PunctuatedAttrs<V> {
    inner: HashMap<Key, (Span, V)>,
}

impl<K, V> TryFrom<Vec<(K, V)>> for PunctuatedAttrs<V>
where
    K: ToKey + Hash + Eq,
{
    type Error = Error;

    fn try_from(value: Vec<(K, V)>) -> Result<Self, Self::Error> {
        let mut inner = HashMap::with_capacity(value.len());

        for (key, value) in value {
            let key = key.to_key();
            let span = match &key {
                Key::Ident(ident) => ident.span(),
                Key::Path(path) => path.span(),
            };

            if inner.contains_key(&key) {
                return Err(Error::new(span, "duplicate attribute"));
            }

            inner.insert(key, (span, value));
        }

        Ok(Self { inner })
    }
}

//-----------------------------------------------------------------------------

#[derive(Debug, Eq, Hash, PartialEq)]
enum Key {
    Ident(Ident),
    Path(Path),
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Ident(ident) => write!(f, "{}", ident),
            Key::Path(path) => write!(f, "{:?}", path),
        }
    }
}

impl TryFrom<MetaNameValue> for Key {
    type Error = Error;

    fn try_from(value: MetaNameValue) -> Result<Self, Self::Error> {
        Ok(value
            .path
            .get_ident()
            .cloned()
            .map_or_else(|| Self::Path(value.path), |ident| Self::Ident(ident)))
    }
}

impl TryFrom<Key> for Ident {
    type Error = Error;

    fn try_from(value: Key) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<Key> for Path {
    type Error = Error;

    fn try_from(value: Key) -> Result<Self, Self::Error> {
        todo!()
    }
}

trait ToKey {
    fn to_key(self) -> Key;
}

impl<V> Parse for PunctuatedAttrs<V>
where
    K: Hash + Eq + Spanned,
    K: ToKey,
    V: TryFrom<syn::Expr>,
{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Punctuated::<Meta, Comma>::parse_terminated(input)?
            .into_iter()
            .map(|meta| match meta {
                Meta::Path(_) | Meta::List(_) => {
                    Err(Error::new(meta.span(), "support only <key> = <value>"))
                }
                Meta::NameValue(name_value) => {
                    let key = TryInto::<Key>::try_into(name_value)?.try_into()?;
                    let value = name_value.value.try_into()?;

                    Ok((key, value))
                    // // Make sure the Path is actually a single Ident
                    // let key_ident = name_value.path.get_ident().cloned().ok_or(syn::Error::new(
                    //     name_value.path.span(),
                    //     "attribute key must be a single identifier",
                    // ))?;
                    //
                    // // Only expect a Path here
                    // let value_ident = if let syn::Expr::Path(ref path) = name_value.value {
                    //     // Same as above, make sure we have a single Ident
                    //     Ok(path.path.clone())
                    // } else {
                    //     Err(syn::Error::new(
                    //         name_value.value.span(),
                    //         "value must be an identifier",
                    //     ))
                    // }?;
                    //
                    // Ok((key_ident, value_ident))
                }
            })
            .collect::<syn::Result<Vec<(_, V)>>>()?
            .try_into()
    }
}

impl<V> PunctuatedAttrs<V> {
    pub(crate) fn iter(&self) -> std::collections::hash_map::Iter<'_, Key, (Span, V)> {
        self.inner.iter()
    }
}

//-----------------------------------------------------------------------------

// Recreate either Ident or Path from str when keys are Ident or Path
impl<V> PunctuatedAttrs<Ident, V> {
    pub(crate) fn remove<Q: AsRef<str>>(&mut self, k: Q) -> Option<(Span, V)> {
        self.inner
            .remove(&Ident::new(k.as_ref(), Span::call_site()))
    }
}

impl<V> PunctuatedAttrs<Path, V> {
    pub(crate) fn remove<Q: AsRef<str>>(&mut self, k: Q) -> Option<(Span, V)> {
        let key: Path = Into::<PathSegment>::into(syn::parse_str::<Ident>(k.as_ref()).ok()?).into();
        self.inner.remove(&key)
    }
}
