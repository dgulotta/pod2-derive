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
                    Self
                }
            }
            syn::Fields::Unnamed(fields) => match fields.unnamed.len() {
                0 => quote! {
                    Self()
                },
                1 => quote! {
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
                    quote! {
                        Self {}
                    }
                }
                else {
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
        impl From<&pod2::middleware::TypedValue> for #name {
            fn from(__v: &pod2::middleware::TypedValue) -> Self {
                #generated
            }
        }
        
        #[automatically_derived]
        impl From<pod2::middleware::TypedValue> for #name {
            fn from(__v: pod2::middleware::TypedValue) -> Self {
                Self::from(&__v)
            }
        }

        #[automatically_derived]
        impl From<&pod2::middleware::Value> for #name {
            fn from(__v: &pod2::middleware::Value) -> Self {
                Self::from(__v.typed())
            }
        }

        #[automatically_derived]
        impl From<pod2::middleware::Value> for #name {
            fn from(__v: pod2::middleware::Value) -> Self {
                Self::from(__v.typed())
            }
        }
    }.into()
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
                    let ident_string = ident.as_ref().unwrap().to_string();
                    let ty = &field.ty;
                    quote! {
                        #ident: <#ty>::try_from(
                            #dict_var.get(&pod2::middleware::Key::from(#ident_string))?.typed())?
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
                        #[automatically_derived]
                        impl TryFrom<&pod2::middleware::TypedValue> for #name {
                            type Error = pod2::middleware::Error;
                            fn try_from(v: &pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                                Ok(Self(v.try_into()?))
                            }
                        }
                    },
                    _ => {
                        let arr_var = format_ident!("__arr");
                        let field_list = fields.unnamed.iter().enumerate().map(|(n, _)| {
                            quote! {
                                #arr_var.array()[#n].typed().try_into()?
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
            let mut direct_list = vec![];
            let mut dict_list = vec![];
            let value_var = format_ident!("__value");
            let key_var = format_ident!("__key");
            let outer_dict_var = format_ident!("__outer_dict");
            let inner_dict_var = format_ident!("__inner_dict");
            let inner_arr_var = format_ident!("__inner_arr");
            for v in e.variants.iter() {
                let variant_ident = &v.ident;
                let variant_string = variant_ident.to_string();
                match &v.fields {
                    Fields::Unit => direct_list.push(quote! {
                        pod2::middleware::TypedValue::String(s) if s == #variant_string => Ok(Self::#variant_ident)
                    }),
                    Fields::Named(fields) => {
                        let struct_field_list = fields.named.iter().map(|field| {
                            let field_ident = &field.ident;
                            let field_string = field_ident.as_ref().unwrap().to_string();
                            let ty = &field.ty;
                            quote! {
                                #field_ident: <#ty>::try_from(
                                    #inner_dict_var.get(&pod2::middleware::Key::from(#field_string))?.typed())?
                        
                            }
                        });
                        dict_list.push(quote! {
                            #variant_string => match #value_var.typed() {
                                pod2::middleware::TypedValue::Dictionary(#inner_dict_var) => {
                                    Ok(Self::#variant_ident {
                                        #(#struct_field_list,)*
                                    })
                                },
                                _ => Err(pod2::middleware::Error::custom(format!("Expected a Dictionary, got {v}")))
                            }
                        });
                    }
                    Fields::Unnamed(fields) => {
                        match fields.unnamed.len() {
                            0 => direct_list.push(quote! {
                                pod2::middleware::TypedValue::String(s) if s == #variant_string => Ok(Self::#variant_ident())
                            }),
                            1 => dict_list.push(quote! {
                                #variant_string => Ok(Self::#variant_ident(#value_var.try_into()?))
                            }),
                            _ => {
                                let field_list = fields.unnamed.iter().enumerate().map(|(n, _)| {
                                    quote! {
                                        #inner_arr_var.array()[#n].typed().try_into()?
                                    }
                                });
                                let num_fields = fields.unnamed.len();
                                dict_list.push(quote! {
                                    #variant_string => {
                                        if let pod2::middleware::TypedValue::Array(#inner_arr_var) = #value_var.typed() {
                                            if #inner_arr_var.array().len() == #num_fields {
                                                Ok(Self::#variant_ident(
                                                    #(#field_list,)*
                                                ))
                                            }
                                            else {
                                                Err(pod2::middleware::Error::custom(
                                                    format!("Expected an Array of length {}, got length {}", #num_fields, #inner_arr_var.array().len())
                                                ))
                                            }
                                        } else {
                                            Err(pod2::middleware::Error::custom(
                                                format!("Expected an Array, got {}", #variant_ident)
                                            ))
                                        }
                                    }
                                });
                            }
                        }
                    },
                }
            }
            let err_msg = match (direct_list.is_empty(), dict_list.is_empty()) {
                (false, false) => "Expected a String or Dictionary with one entry",
                (true, false) => "Expected a Dictionary with one entry",
                (false, true) => "Expected a String",
                (true, true) => "Cannot instantiate an empty enum",
            };
            if !dict_list.is_empty() {
                direct_list.push(quote! {
                    pod2::middleware::TypedValue::Dictionary(#outer_dict_var) if #outer_dict_var.kvs().len() == 1 => {
                        let (#key_var, #value_var) = #outer_dict_var.kvs().iter().next().unwrap();
                        match #key_var {
                            #(#dict_list,)*
                        }
                    }
                });
            }
            direct_list.push(quote!{
                _ => Err(pod2::middleware::Error::custom(#err_msg.to_string()))
            });
            quote! {
                #[automatically_derived]
                impl TryFrom<&pod2::middleware::TypedValue> for #name {
                    type Error = pod2::middleware::Error;
                    fn try_from(v: &pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                        match v {
                            #(#direct_list,)*
                        }
                    }
                }
            }
        },
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
