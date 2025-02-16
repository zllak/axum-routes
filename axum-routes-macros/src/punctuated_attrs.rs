use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::Deref,
};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    Error, Expr, Ident, Meta, MetaNameValue, Path,
};

/// The attributes in a #[...]
/// ie: handler = some_handler, id = Ty
#[derive(Debug)]
pub(crate) struct PunctuatedAttrs<K, V> {
    inner: HashMap<Key<K>, Value<V>>,
}

impl<K, V> Default for PunctuatedAttrs<K, V> {
    fn default() -> Self {
        Self {
            inner: HashMap::default(),
        }
    }
}

//-----------------------------------------------------------------------------

impl<K, V> Parse for PunctuatedAttrs<K, V>
where
    K: Hash + Eq + Clone,
    Key<K>: TryFrom<MetaNameValue, Error = Error>,
    Value<V>: TryFrom<Expr, Error = Error>,
{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut inner = HashSet::<Key<K>>::default();

        Ok(Self {
            inner: Punctuated::<Meta, Comma>::parse_terminated(input)?
                .into_iter()
                .map(|meta| match meta {
                    Meta::Path(_) | Meta::List(_) => {
                        Err(Error::new(meta.span(), "support only <key> = <value>"))
                    }
                    Meta::NameValue(meta) => {
                        let span = meta.span();
                        let key = TryInto::<Key<K>>::try_into(meta.clone())?;
                        let value = TryInto::<Value<V>>::try_into(meta.value)?;

                        if inner.contains(&key) {
                            return Err(Error::new(span, "duplicate attribute"));
                        }
                        inner.insert(key.clone());

                        Ok((key, value))
                    }
                })
                .collect::<syn::Result<HashMap<_, _>>>()?,
        })
    }
}

impl<K, V> PunctuatedAttrs<K, V> {
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.inner.iter().map(|(k, v)| (k.deref(), v.deref()))
    }
}

impl<K, V> PunctuatedAttrs<K, V>
where
    K: Hash + Eq + PartialEq + Parse,
{
    pub(crate) fn remove(&mut self, k: impl AsRef<str>) -> Option<V> {
        self.inner
            .remove(&Key(syn::parse_str::<K>(k.as_ref()).ok()?))
            .map(|value| value.0)
    }
}

//-----------------------------------------------------------------------------

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
struct Key<K>(K);

impl TryFrom<MetaNameValue> for Key<Ident> {
    type Error = Error;

    fn try_from(value: MetaNameValue) -> Result<Self, Self::Error> {
        Ok(Self(value.path.get_ident().cloned().ok_or(
            syn::Error::new(value.span(), "expected a single-value ident"),
        )?))
    }
}

impl TryFrom<MetaNameValue> for Key<Path> {
    type Error = Error;

    fn try_from(value: MetaNameValue) -> Result<Self, Self::Error> {
        Ok(Self(value.path))
    }
}

impl<K> Deref for Key<K> {
    type Target = K;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

//-----------------------------------------------------------------------------

#[derive(Debug)]
struct Value<V>(V);

impl TryFrom<Expr> for Value<Ident> {
    type Error = Error;

    fn try_from(value: Expr) -> Result<Self, Self::Error> {
        if let Expr::Path(syn::ExprPath { path, .. }) = value {
            path.require_ident().cloned().map(Self)
        } else {
            Err(Error::new(value.span(), "expected a single value ident"))
        }
    }
}

impl TryFrom<Expr> for Value<Path> {
    type Error = Error;

    fn try_from(value: Expr) -> Result<Self, Self::Error> {
        if let Expr::Path(syn::ExprPath { path, .. }) = value {
            Ok(Self(path))
        } else {
            Err(Error::new(value.span(), "expected a path"))
        }
    }
}

impl TryFrom<Expr> for Value<Expr> {
    type Error = Error;

    fn try_from(value: Expr) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

impl<V> Deref for Value<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
