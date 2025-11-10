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
///         self.current + 1
///     }
///
///     const fn name(&self) -> &str {
///         "Index"
///     }
/// }
///
/// fn main() {
///     let idx = Index { current: 5 };
///     let value = serde_json::to_value(&idx).unwrap();
///     assert_eq!(value, json!({
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

    let mut extra_pairs: Vec<(String, String)> = Vec::new();
    for attr in &input.attrs {
        match parse_more_attribute(attr) {
            Ok(Some((k, v))) => extra_pairs.push((k, v)),
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

    let extra_keys: Vec<_> = extra_pairs.iter().map(|(k, _)| k).collect();
    let extra_methods: Vec<_> = extra_pairs
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

                // Custom serializer that captures fields and adds extras
                // TODO
                // This should be moved outside of the macro
                // but this will require a separate crate that is not a proc-macro.
                struct FlatMapSerializer<'a, M: SerializeMap> {
                    map: &'a mut M,
                }

                impl<'a, M: SerializeMap> Serializer for FlatMapSerializer<'a, M> {
                    type Ok = ();
                    type Error = M::Error;

                    type SerializeSeq = serde::ser::Impossible<(), M::Error>;
                    type SerializeTuple = serde::ser::Impossible<(), M::Error>;
                    type SerializeTupleStruct = serde::ser::Impossible<(), M::Error>;
                    type SerializeTupleVariant = serde::ser::Impossible<(), M::Error>;
                    type SerializeMap = serde::ser::Impossible<(), M::Error>;
                    type SerializeStruct = Self;
                    type SerializeStructVariant = serde::ser::Impossible<(), M::Error>;

                    fn serialize_struct(
                        self,
                        _name: &'static str,
                        _len: usize,
                    ) -> Result<Self::SerializeStruct, M::Error> {
                        Ok(self)
                    }

                    fn serialize_bool(self, _v: bool) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_i8(self, _v: i8) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_i16(self, _v: i16) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_i32(self, _v: i32) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_i64(self, _v: i64) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_u8(self, _v: u8) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_u16(self, _v: u16) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_u32(self, _v: u32) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_u64(self, _v: u64) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_f32(self, _v: f32) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_f64(self, _v: f64) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_char(self, _v: char) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_str(self, _v: &str) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_bytes(self, _v: &[u8]) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_none(self) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_some<T: ?Sized + serde::Serialize>(self, _value: &T) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_unit(self) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_unit_variant(
                        self,
                        _name: &'static str,
                        _variant_index: u32,
                        _variant: &'static str,
                    ) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_newtype_struct<T: ?Sized + serde::Serialize>(
                        self,
                        _name: &'static str,
                        _value: &T,
                    ) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_newtype_variant<T: ?Sized + serde::Serialize>(
                        self,
                        _name: &'static str,
                        _variant_index: u32,
                        _variant: &'static str,
                        _value: &T,
                    ) -> Result<(), M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_tuple_struct(
                        self,
                        _name: &'static str,
                        _len: usize,
                    ) -> Result<Self::SerializeTupleStruct, M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_tuple_variant(
                        self,
                        _name: &'static str,
                        _variant_index: u32,
                        _variant: &'static str,
                        _len: usize,
                    ) -> Result<Self::SerializeTupleVariant, M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }

                    fn serialize_struct_variant(
                        self,
                        _name: &'static str,
                        _variant_index: u32,
                        _variant: &'static str,
                        _len: usize,
                    ) -> Result<Self::SerializeStructVariant, M::Error> {
                        Err(serde::ser::Error::custom("expected struct"))
                    }
                }

                impl<'a, M: SerializeMap> serde::ser::SerializeStruct for FlatMapSerializer<'a, M> {
                    type Ok = ();
                    type Error = M::Error;

                    fn serialize_field<T: ?Sized + serde::Serialize>(
                        &mut self,
                        key: &'static str,
                        value: &T,
                    ) -> Result<(), M::Error> {
                        self.map.serialize_entry(key, value)
                    }

                    fn end(self) -> Result<(), M::Error> {
                        Ok(())
                    }
                }

                let mut map = serializer.serialize_map(None)?;
                {
                    let flat = FlatMapSerializer {
                        map: &mut map,
                    };
                    helper.serialize(flat)?;
                }

                #(
                    map.serialize_entry(#extra_keys, &#extra_methods)?;
                )*

                map.end()
            }
        }
    };

    TokenStream::from(serialize_impl)
}

fn parse_more_attribute(attr: &Attribute) -> syn::Result<Option<(String, String)>> {
    if !attr.path().is_ident("more") {
        return Ok(None);
    }

    let mut key: Option<String> = None;
    let mut value: Option<String> = None;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("key") || meta.path.is_ident("k") {
            let lit: LitStr = meta.value()?.parse()?;
            key = Some(lit.value());
            Ok(())
        } else if meta.path.is_ident("value") || meta.path.is_ident("v") {
            let lit: LitStr = meta.value()?.parse()?;
            value = Some(lit.value());
            Ok(())
        } else {
            Err(meta.error("unsupported attribute key, expected 'key', 'k', 'value', or 'v'"))
        }
    })?;

    match (key, value) {
        (Some(k), Some(v)) => Ok(Some((k, v))),
        (Some(k), None) => Ok(Some((k.clone(), k))),
        _ => Ok(None),
    }
}
