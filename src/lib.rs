use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

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
            _ => todo!(),
        },
        _ => todo!(),
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
