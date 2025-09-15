use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{DeriveInput, Fields, spanned::Spanned};

fn unions_not_supported(span: Span) -> TokenStream {
    syn::Error::new(span.into(), "pod2_derive does not support unions")
        .to_compile_error()
        .into()
}

#[proc_macro_derive(FromValue)]
pub fn from_value_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let generated = match &ast.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Unit => {
                quote! {
                    #[automatically_derived]
                    impl From<&pod2::middleware::TypedValue> for #name {
                        fn from(_: &pod2::middleware::TypedValue) -> Self {
                            Self
                        }
                    }
                }
            }
            syn::Fields::Unnamed(fields) => match fields.unnamed.len() {
                0 => todo!(),
                1 => todo!(),
                _ => {
                    return syn::Error::new(
                        ast.span(),
                        "Cannot derive FromValue for a tuple struct with more than one field",
                    )
                    .to_compile_error()
                    .into();
                }
            },
            syn::Fields::Named(_) => {
                return syn::Error::new(
                    ast.span(),
                    "Cannot derive FromValue for a struct with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        syn::Data::Enum(_) => {
            return syn::Error::new(ast.span(), "Cannot derive FromValue for an enum")
                .to_compile_error()
                .into();
        }
        syn::Data::Union(_) => return unions_not_supported(ast.span()),
    };
    let forward = quote! {
        #[automatically_derived]
        impl From<pod2::middleware::TypedValue> for #name {
            fn from(v: pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                Self::from(&v)
            }
        }

        #[automatically_derived]
        impl From<&pod2::middleware::Value> for #name {
            fn from(v: &pod2::middleware::Value) -> Result<Self, Self::Error> {
                Self::from(v.typed())
            }
        }

        #[automatically_derived]
        impl From<pod2::middleware::Value> for #name {
            fn from(v: pod2::middleware::Value) -> Result<Self, Self::Error> {
                Self::from(v.typed())
            }
        }
    };
    quote! {
        #generated
        #forward
    }
    .into()
}

#[proc_macro_derive(TryFromValue)]
pub fn try_from_value_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let generated = match ast.data {
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Named(fields) => {
                let dict_var = format_ident!("__dict");
                let field_list = fields.named.iter().map(|field| {
                    let ident = &field.ident;
                    let ty = &field.ty;
                    quote! {
                        #ident: <#ty>::try_from(
                            #dict_var.get(&pod2::middleware::Key::from(stringify!(#ident)))?.typed())?
                    }
                });
                quote! {
                    #[automatically_derived]
                    impl TryFrom<&pod2::middleware::TypedValue> for #name {
                        type Error = pod2::middleware::Error;
                        fn try_from(v: &pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                            if let pod2::middleware::TypedValue::Dictionary(#dict_var) = v {
                                Ok(Self {
                                    #(#field_list,)*
                                })
                            } else {
                                Err(pod2::middleware::Error::custom(format!("Expected a Dictionary, got {v}")))
                            }
                       }
                   }
                }
            }
            syn::Fields::Unnamed(fields) => {
                let num_fields = fields.unnamed.len();
                match num_fields {
                    0 => quote! {
                        #[automatically_derived]
                        impl TryFrom<&pod2::middleware::TypedValue> for #name {
                            type Error = pod2::middleware::Error;
                            fn try_from(_: &pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                                Ok(Self())
                            }
                        }
                    },
                    1 => quote! {
                        impl TryFrom<&pod2::middleware::TypedValue> for #name {
                            type Error = pod2::middleware::Error;
                            fn try_from(v: &pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                                Ok(Self(v.try_into()?))
                            }
                        }
                    },
                    _ => {
                        let arr_var = format_ident!("__arr");
                        let field_list = fields.unnamed.iter().enumerate().map(|(n, field)| {
                            let ty = &field.ty;
                            quote! {
                                <#ty>::try_from(#arr_var.array()[#n].typed())?
                            }
                        });
                        quote! {
                            #[automatically_derived]
                            impl TryFrom<&pod2::middleware::TypedValue> for #name {
                                type Error = pod2::middleware::Error;
                                fn try_from(v: &pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                                    if let pod2::middleware::TypedValue::Array(#arr_var) = v {
                                        if #arr_var.array().len() == #num_fields {
                                            Ok(Self (
                                                #(#field_list,)*
                                            ))
                                        }
                                        else {
                                            Err(pod2::middleware::Error::custom(
                                                format!("Expected an Array of length {}, got length {}", #num_fields, #arr_var.array().len())
                                            ))
                                        }
                                    } else {
                                        Err(pod2::middleware::Error::custom(
                                            format!("Expected an Array, got {v}")
                                        ))
                                    }
                                }
                            }
                        }
                    }
                }
            }
            syn::Fields::Unit => {
                quote! {
                    #[automatically_derived]
                    impl TryFrom<&pod2::middleware::TypedValue> for #name {
                        type Error = pod2::middleware::Error;
                        fn try_from(_: &pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                            Ok(Self)
                        }
                    }
                }
            }
        },
        syn::Data::Enum(e) => {
            let variant_list = e.variants.iter().map(|v| {
                let name = &v.ident;
                match v.fields {
                    Fields::Unit => quote! {
                        pod2::middleware::TypedValue::String(s) if s == #name => Ok(#name)
                    },
                    _ => todo!(),
                }
            });
            todo!()
        }
        syn::Data::Union(_) => return unions_not_supported(ast.span()),
    };
    let forward = quote! {
        #[automatically_derived]
        impl TryFrom<pod2::middleware::TypedValue> for #name {
            type Error = pod2::middleware::Error;
            fn try_from(v: pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                Self::try_from(&v)
            }
        }

        #[automatically_derived]
        impl TryFrom<&pod2::middleware::Value> for #name {
            type Error = pod2::middleware::Error;
            fn try_from(v: &pod2::middleware::Value) -> Result<Self, Self::Error> {
                Self::try_from(v.typed())
            }
        }

        #[automatically_derived]
        impl TryFrom<pod2::middleware::Value> for #name {
            type Error = pod2::middleware::Error;
            fn try_from(v: pod2::middleware::Value) -> Result<Self, Self::Error> {
                Self::try_from(v.typed())
            }
        }
    };
    quote! {
        #generated
        #forward
    }
    .into()
}
