#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;

use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, LitStr, parse_macro_input};

/// A derive macro to implement [`serde::Serialize`] with arbitrary extra fields specified via
/// `#[more(key="...", value="...")]` attributes. The `value` should be a method on the struct that
/// returns a type implementing `serde::Serialize`.
///
/// Works with `serde` and `serde_with` attributes on the struct and its fields.
///
/// ## Limitations
///
/// - Currently only supports structs with named fields.
///
/// ## Example
///
/// ```rust
/// use serde_more::SerializeMore;
/// use serde_json::json;
///
/// #[derive(SerializeMore)]
/// // Add field `next` with value from method `get_next()`.
/// #[more(key="next", value="get_next")]
/// // `k` and `v` are valid as shorthand for `key` and `value`.
/// #[more(k="description", v="get_description")]
/// // If only key is provided, value method is assumed to have the same name.
/// #[more(k="name")]
/// // Use `position="front"` to put a field before regular fields.
/// // Default is "back" (after regular fields).
/// #[more(key="previous", position="front")]
/// struct Index {
///     current: u32,
/// }
///
/// impl Index {
///     const fn get_description(&self) -> &str {
///         "Index struct"
///     }
///
///     fn get_next(&self) -> u32 {
///         self.current.saturating_add(1)
///     }
///
///     const fn name(&self) -> &str {
///         "Index"
///     }
///
///     fn previous(&self) -> u32 {
///         self.current.saturating_sub(1)
///     }
/// }
///
/// fn main() {
///     let idx = Index { current: 5 };
///     let value = serde_json::to_value(&idx).unwrap();
///     assert_eq!(value, json!({
///         "previous": 4,
///         "current": 5,
///         "next": 6,
///         "description": "Index struct",
///         "name": "Index"
///     }));
/// }
/// ```
///
#[proc_macro_derive(SerializeMore, attributes(more, serde))]
pub fn serialize_more_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let mut front_pairs: Vec<(String, String)> = Vec::new();
    let mut back_pairs: Vec<(String, String)> = Vec::new();
    for attr in &input.attrs {
        match parse_more_attribute(attr) {
            Ok(Some((k, v, true))) => front_pairs.push((k, v)),
            Ok(Some((k, v, false))) => back_pairs.push((k, v)),
            Ok(None) => {}
            Err(e) => return TokenStream::from(e.to_compile_error()),
        }
    }

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "`SerializeMore` only supports structs with named fields.",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "`SerializeMore` only supports structs.")
                .to_compile_error()
                .into();
        }
    };

    let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let field_attrs: Vec<_> = fields.iter().map(|f| &f.attrs).collect();

    let struct_attrs: Vec<_> = input
        .attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("more"))
        .collect();

    let helper_fields = field_names
        .iter()
        .zip(field_types.iter())
        .zip(field_attrs.iter())
        .map(|((name, ty), attrs)| {
            quote! {
                #(#attrs)*
                #name: &'a #ty
            }
        });

    let field_assignments = field_names.iter().map(|name| {
        quote! { #name: &self.#name }
    });

    let front_keys: Vec<_> = front_pairs.iter().map(|(k, _)| k).collect();
    let front_methods: Vec<_> = front_pairs
        .iter()
        .map(|(_, v)| {
            let method_ident = syn::Ident::new(v, proc_macro2::Span::call_site());
            quote! { self.#method_ident() }
        })
        .collect();

    let back_keys: Vec<_> = back_pairs.iter().map(|(k, _)| k).collect();
    let back_methods: Vec<_> = back_pairs
        .iter()
        .map(|(_, v)| {
            let method_ident = syn::Ident::new(v, proc_macro2::Span::call_site());
            quote! { self.#method_ident() }
        })
        .collect();

    let serialize_impl = quote! {
        impl serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::{Serializer, SerializeMap};

                #[derive(serde::Serialize)]
                #(#struct_attrs)*
                struct Helper<'a> {
                    #(#helper_fields,)*
                }

                let helper = Helper {
                    #(#field_assignments,)*
                };

                let mut map = serializer.serialize_map(None)?;

                #(
                    map.serialize_entry(#front_keys, &#front_methods)?;
                )*

                {
                    let flat = serde_more::FlatMapSerializer {
                        map: &mut map,
                    };
                    helper.serialize(flat)?;
                }

                #(
                    map.serialize_entry(#back_keys, &#back_methods)?;
                )*

                map.end()
            }
        }
    };

    TokenStream::from(serialize_impl)
}

fn parse_more_attribute(attr: &Attribute) -> syn::Result<Option<(String, String, bool)>> {
    if !attr.path().is_ident("more") {
        return Ok(None);
    }

    let mut key: Option<String> = None;
    let mut value: Option<String> = None;
    let mut is_front: bool = false;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("key") || meta.path.is_ident("k") {
            let lit: LitStr = meta.value()?.parse()?;
            key = Some(lit.value());
            Ok(())
        } else if meta.path.is_ident("value") || meta.path.is_ident("v") {
            let lit: LitStr = meta.value()?.parse()?;
            value = Some(lit.value());
            Ok(())
        } else if meta.path.is_ident("position") {
            let lit: LitStr = meta.value()?.parse()?;
            is_front = match lit.value().to_ascii_lowercase().as_str() {
                "front" => true,
                "back" => false,
                invalid => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        format!("invalid position '{invalid}', expected 'front' or 'back'"),
                    ));
                }
            };
            Ok(())
        } else {
            Err(meta.error(
                "unsupported attribute key, expected 'key', 'k', 'value', 'v', or 'position'",
            ))
        }
    })?;

    match (key, value) {
        (Some(k), Some(v)) => Ok(Some((k, v, is_front))),
        (Some(k), None) => Ok(Some((k.clone(), k, is_front))),
        _ => Ok(None),
    }
}
