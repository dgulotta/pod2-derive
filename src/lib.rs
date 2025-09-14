use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

#[proc_macro_derive(TryFromValue)]
pub fn try_from_value_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let (generated, err_type) = match ast.data {
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Named(fields) => {
                let err_type = quote! { pod2::middleware::Error };
                let dict_var = format_ident!("__dict");
                let field_list = fields.named.iter().map(|field| {
                    let ident = &field.ident;
                    let ty = &field.ty;
                    quote! {
                        #ident: <#ty>::try_from(
                            #dict_var.get(&pod2::middleware::Key::from(stringify!(#ident)))?.typed())?
                    }
                });
                let generated = quote! {
                    #[automatically_derived]
                    impl TryFrom<&pod2::middleware::TypedValue> for #name {
                        type Error = #err_type;
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
                };
                (generated, err_type)
            }
            syn::Fields::Unnamed(fields) => {
                let err_type = quote! { pod2::middleware::Error };
                let arr_var = format_ident!("__arr");
                let field_list = fields.unnamed.iter().enumerate().map(|(n, field)| {
                    let ty = &field.ty;
                    quote! {
                        <#ty>::try_from(#arr_var.array()[#n].typed())?
                    }
                });
                let num_fields = fields.unnamed.len();
                let generated = quote! {
                    #[automatically_derived]
                    impl TryFrom<&pod2::middleware::TypedValue> for #name {
                        type Error = #err_type;
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
                };
                (generated, err_type)
            }
            syn::Fields::Unit => {
                let err_type = quote! { core::convert::Infallible };
                let generated = quote! {
                    #[automatically_derived]
                    impl From<&pod2::middleware::TypedValue> for #name {
                        fn from(_: &pod2::middleware::TypedValue) -> Self {
                            Self {}
                        }
                    }
                };
                (generated, err_type)
            }
        },
        _ => todo!(),
    };
    let forward = quote! {
        #[automatically_derived]
        impl TryFrom<pod2::middleware::TypedValue> for #name {
            type Error = #err_type;
            fn try_from(v: pod2::middleware::TypedValue) -> Result<Self, Self::Error> {
                Self::try_from(&v)
            }
        }

        #[automatically_derived]
        impl TryFrom<&pod2::middleware::Value> for #name {
            type Error = #err_type;
            fn try_from(v: &pod2::middleware::Value) -> Result<Self, Self::Error> {
                Self::try_from(v.typed())
            }
        }

        #[automatically_derived]
        impl TryFrom<pod2::middleware::Value> for #name {
            type Error = #err_type;
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
