use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    DataEnum, DeriveInput, Expr, Fields, FieldsNamed, FieldsUnnamed, Path, parse_quote,
    spanned::Spanned,
};

fn unions_not_supported(span: Span) -> proc_macro::TokenStream {
    syn::Error::new(span.into(), "pod2_derive does not support unions")
        .to_compile_error()
        .into()
}

fn constructor(variant: Option<&Ident>) -> Path {
    match variant {
        Some(var) => parse_quote! { Self::#var },
        None => parse_quote! { Self },
    }
}

fn struct_from_dict_fn(fields: &FieldsNamed, variant: Option<&Ident>) -> Expr {
    let field_list = fields.named.iter().map(|field| {
        let ident = &field.ident;
        let ident_string = ident.as_ref().unwrap().to_string();
        let ty = &field.ty;
        quote! {
            #ident: <#ty>::try_from(
                __d.get(&::pod2::middleware::Key::from(#ident_string))?.typed())?
        }
    });
    let constructor = constructor(variant);
    parse_quote! {
        |__d: &::pod2::middleware::containers::Dictionary| -> ::pod2::middleware::Result<Self> {
            ::core::result::Result::Ok(#constructor {
                #(#field_list,)*
            })
        }
    }
}

fn tuple_struct_from_array_fn(fields: &FieldsUnnamed, variant: Option<&Ident>) -> Expr {
    let expected_len = fields.unnamed.len();
    let field_list = fields.unnamed.iter().enumerate().map(|(n, _)| {
        quote! {
            __a.array()[#n].typed().try_into()?
        }
    });
    let constructor = constructor(variant);
    parse_quote! {
        |__a: &::pod2::middleware::containers::Array| -> ::pod2::middleware::Result<Self> {
            if __a.array().len() == #expected_len {
                ::core::result::Result::Ok(#constructor (
                    #(#field_list,)*
                ))
            } else {
                return ::core::result::Result::Err(::pod2::middleware::Error::custom(format!("Expected an Array of length #expected_len, got length {}", __a.array().len())));
            }
        }
    }
}

fn unit_variant(variant: &Ident) -> TokenStream {
    let variant_string = variant.to_string();
    quote! {
        ::pod2::middleware::TypedValue::String(__s) if __s == #variant_string => Self::#variant
    }
}

fn enum_from_value_fn(e: &DataEnum) -> Expr {
    let mut direct_list = vec![];
    let mut dict_list = vec![];
    for v in e.variants.iter() {
        match &v.fields {
            Fields::Unit => direct_list.push(unit_variant(&v.ident)),
            Fields::Unnamed(fields) => {
                let closure = tuple_struct_from_array_fn(fields, Some(&v.ident));
                let variant_string = v.ident.to_string();
                dict_list.push(quote! {
                    #variant_string => {
                        let __variant_from_array = #closure;
                        __variant_from_array(__value.typed().try_into()?)?
                    }
                });
            }
            Fields::Named(fields) => {
                let closure = struct_from_dict_fn(fields, Some(&v.ident));
                let variant_string = v.ident.to_string();
                dict_list.push(quote! {
                    #variant_string => {
                        let __variant_from_dict = #closure;
                        __variant_from_dict(__value.typed().try_into()?)?
                    }
                });
            }
        }
    }
    let err_msg = match (direct_list.is_empty(), dict_list.is_empty()) {
        (false, false) => "Expected a String or Dictionary with one entry",
        (true, false) => "Expected a Dictionary with one entry",
        (false, true) => "Expected a String",
        (true, true) => "Cannot instantiate an empty enum",
    };
    if !dict_list.is_empty() {
        dict_list.push(quote! {
            _ => return ::core::result::Result::Err(::pod2::middleware::Error::custom(format!("Expected name of tuple or struct variant, got {__key}")))
        });
        direct_list.push(quote! {
            ::pod2::middleware::TypedValue::Dictionary(__d) if __d.kvs().len() == 1 => {
                let (__key, __value) = __d.kvs().iter().next().unwrap();
                match __key.name() {
                    #(#dict_list,)*
                }
            }
        });
    }
    direct_list.push(quote! {
        _ => return ::core::result::Result::Err(::pod2::middleware::Error::custom(#err_msg.to_string()))
    });
    // the Ok is unreachable if the enum is empty
    parse_quote! {
        |__v: &::pod2::middleware::TypedValue| -> ::pod2::middleware::Result<Self> {
            #[allow(unreachable_code)]
            ::core::result::Result::Ok(match __v {
                #(#direct_list,)*
            })
        }
    }
}

fn dict_from_struct_fn(fields: &FieldsNamed) -> Expr {
    let insert_statements = fields.named.iter().map(|field| {
        let ident = &field.ident;
        let ident_string = ident.as_ref().unwrap().to_string();
        quote! {
            __kvs.insert(::pod2::middleware::Key::from(#ident_string), __v.#ident.into());
        }
    });
    parse_quote! {
        |__x: &Self, __params: &::pod2::middleware::Params| -> ::pod2::middleware::Result<::pod2::middleware::Dictionary> {
            let __kvs = ::std::collections::HashMap::new();
            #(#insert_statements)*
            ::pod2::middleware::Dictionary::new(__params.max_depth_mt_containers, __kvs)
        }
    }
}

#[proc_macro_derive(FromValue)]
pub fn from_value_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let generated: Expr = match &ast.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Unit => {
                parse_quote! {
                    Self
                }
            }
            syn::Fields::Unnamed(fields) => match fields.unnamed.len() {
                0 => parse_quote! {
                    Self()
                },
                1 => parse_quote! {
                    Self(__v.into())
                },
                _ => {
                    return syn::Error::new(
                        ast.span(),
                        "Cannot derive FromValue for a tuple struct with more than one field",
                    )
                    .to_compile_error()
                    .into();
                }
            },
            syn::Fields::Named(fields) => {
                if fields.named.is_empty() {
                    parse_quote! {
                        Self {}
                    }
                } else {
                    return syn::Error::new(
                        ast.span(),
                        "Cannot derive FromValue for a struct with named fields",
                    )
                    .to_compile_error()
                    .into();
                }
            }
        },
        syn::Data::Enum(_) => {
            return syn::Error::new(ast.span(), "Cannot derive FromValue for an enum")
                .to_compile_error()
                .into();
        }
        syn::Data::Union(_) => return unions_not_supported(ast.span()),
    };
    quote! {
        #[automatically_derived]
        impl ::core::convert::From<&::pod2::middleware::TypedValue> for #name {
            fn from(__v: &::pod2::middleware::TypedValue) -> Self {
                #generated
            }
        }

        #[automatically_derived]
        impl ::core::convert::From<::pod2::middleware::TypedValue> for #name {
            fn from(__v: ::pod2::middleware::TypedValue) -> Self {
                Self::from(&__v)
            }
        }

        #[automatically_derived]
        impl ::core::convert::From<&::pod2::middleware::Value> for #name {
            fn from(__v: &::pod2::middleware::Value) -> Self {
                Self::from(__v.typed())
            }
        }

        #[automatically_derived]
        impl ::core::convert::From<::pod2::middleware::Value> for #name {
            fn from(__v: ::pod2::middleware::Value) -> Self {
                Self::from(__v.typed())
            }
        }
    }
    .into()
}

#[proc_macro_derive(IntoValue)]
pub fn into_value_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let generated: Expr = match &ast.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                parse_quote! {
                    __v.0.into()
                }
            }
            _ => {
                return syn::Error::new(
                    ast.span(),
                    "Cannot derive IntoValue for non-newtype struct",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => todo!(),
    };
    quote! {
        impl From<&#name> for ::pod2::middleware::TypedValue {
            fn from(__v: &#name) -> Self {
                #generated
            }
        }

        impl From<#name> for ::pod2::middleware::TypedValue {
            fn from(__v: #name) -> Self {
                (&__v).into()
            }
        }

    }
    .into()
}

#[proc_macro_derive(PodTryIntoValue)]
pub fn pod_try_into_value_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let generated = match ast.data {
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Named(fields) => {
                let closure = dict_from_struct_fn(&fields);
                quote! {
                    let __dict_from_struct = #closure;
                    ::core::result::Result::Ok(::pod2::middleware::TypedValue::Dictionary(__dict_from_struct(__v)?))
                }
            }
            _ => todo!(),
        },
        _ => todo!(),
    };
    quote! {
        #[automatically_derived]
        impl ::pod2::middleware::convert::PodTryInto<::pod2::middleware::TypedValue> for &#name {
            type Error = ::pod2::middleware::Error;
            fn pod_try_into(self, __params: &::pod2::middleware::Params) {
                #generated
            }
        }

        #[automatically_derived]
        impl ::pod2::middleware::convert::PodTryInto<::pod2::middleware::TypedValue> for #name {
            type Error = ::pod2::middleware::Error;
            fn pod_try_into(self, __params: &::pod2::middleware::Params) {
                (&name).pod_try_into(__params)
            }
        }

        impl ::pod2::middleware::convert::PodTryInto<::pod2::middleware::Value> for &#name {
            type Error = ::pod2::middleware::Error;
            fn pod_try_into(self, __params: &::pod2::middleware::Params) {
                let __tmp: ::pod2::middleware::TypedValue = self.pod_try_into(__params)?;
                ::core::result::Result::Ok(pod2::middleware::Value::from(__tmp))
            }
        }

        #[automatically_derived]
        impl ::pod2::middleware::convert::PodTryInto<::pod2::middleware::Value> for #name {
            type Error = ::pod2::middleware::Error;
            fn pod_try_into(self, __params: &::pod2::middleware::Params) {
                (&name).pod_try_into(__params)
            }
        }
    }
    .into()
}

#[proc_macro_derive(TryFromValue)]
pub fn try_from_value_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let generated: Expr = match ast.data {
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Named(fields) => {
                if fields.named.is_empty() {
                    parse_quote! { ::core::result::Result::Ok(Self {}) }
                } else {
                    let closure = struct_from_dict_fn(&fields, None);
                    parse_quote! {
                        {
                            let __struct_from_dict = #closure;
                            __struct_from_dict(__v.try_into()?)
                        }
                    }
                }
            }
            syn::Fields::Unnamed(fields) => {
                let num_fields = fields.unnamed.len();
                match num_fields {
                    0 => parse_quote! { ::core::result::Result::Ok(Self()) },
                    1 => parse_quote! { ::core::result::Result::Ok(Self(__v.try_into()?)) },
                    _ => {
                        let closure = tuple_struct_from_array_fn(&fields, None);
                        parse_quote! {
                            {
                                let __struct_from_array = #closure;
                                __struct_from_array(__v.try_into()?)
                            }
                        }
                    }
                }
            }
            syn::Fields::Unit => {
                parse_quote! { ::core::result::Result::Ok(Self) }
            }
        },
        syn::Data::Enum(e) => {
            let closure = enum_from_value_fn(&e);
            parse_quote! {
                {
                    let __enum_from_value = #closure;
                    __enum_from_value(__v)
                }
            }
        }
        syn::Data::Union(_) => return unions_not_supported(ast.span()),
    };
    quote! {
        #[automatically_derived]
        impl ::core::convert::TryFrom<&::pod2::middleware::TypedValue> for #name {
            type Error = ::pod2::middleware::Error;
            fn try_from(__v: &::pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                #generated
            }
        }

        #[automatically_derived]
        impl ::core::convert::TryFrom<::pod2::middleware::TypedValue> for #name {
            type Error = ::pod2::middleware::Error;
            fn try_from(__v: ::pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                Self::try_from(&__v)
            }
        }

        #[automatically_derived]
        impl ::core::convert::TryFrom<&::pod2::middleware::Value> for #name {
            type Error = ::pod2::middleware::Error;
            fn try_from(__v: &::pod2::middleware::Value) -> Result<Self, Self::Error> {
                Self::try_from(__v.typed())
            }
        }

        #[automatically_derived]
        impl ::core::convert::TryFrom<::pod2::middleware::Value> for #name {
            type Error = ::pod2::middleware::Error;
            fn try_from(__v: ::pod2::middleware::Value) -> Result<Self, Self::Error> {
                Self::try_from(__v.typed())
            }
        }
    }
    .into()
}
