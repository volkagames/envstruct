mod default_attr;
mod normalize_type_path;

use darling::{ast, FromDeriveInput, FromField};
use default_attr::*;
use normalize_type_path::*;
use proc_macro::TokenStream;
use quote::*;
use syn::spanned::Spanned;

#[proc_macro_derive(EnvStruct, attributes(env))]
pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input: syn::DeriveInput = syn::parse(input).expect("Failed to parse derive input");
    let receiver = EnvStructInputReceiver::from_derive_input(&derive_input)
        .expect("Failed to parse input for darling receiver");
    quote!(#receiver).into()
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(EnvStruct), supports(any))]
struct EnvStructInputReceiver {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<(), EnvStructFieldReceiver>,
}

#[derive(Debug, FromField)]
#[darling(attributes(env))]
struct EnvStructFieldReceiver {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    name: Option<String>,
    default: Option<DefaultAttr>,
    with: Option<syn::Expr>,
    #[darling(default)]
    flatten: bool,
    #[darling(default)]
    skip: bool,
}

impl EnvStructFieldReceiver {
    pub fn name_exr(&self, index: usize) -> proc_macro2::TokenStream {
        self.ident
            .as_ref()
            .map(quote::ToTokens::to_token_stream)
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(#index)
            })
    }

    pub fn type_expr(&self) -> proc_macro2::TokenStream {
        self.with
            .as_ref()
            .map(|ty| quote_spanned! { ty.span() => #ty })
            .unwrap_or({
                let ty = normalize_type_path(&self.ty);
                quote_spanned! { ty.span() => #ty }
            })
    }

    pub fn default_expr(&self) -> proc_macro2::TokenStream {
        self.default
            .as_ref()
            .map(|default| match default {
                DefaultAttr::String(str) => {
                    quote!(Some(#str))
                }
                DefaultAttr::Type(typ) => {
                    quote!(Some(&#typ.to_string()))
                }
                DefaultAttr::Default => {
                    let ty = normalize_type_path(&self.ty);
                    quote!(Some(&#ty::default().to_string()))
                }
            })
            .unwrap_or_else(|| quote!(None))
    }

    pub fn var_name_expr(&self) -> proc_macro2::TokenStream {
        let var_name = self.name.clone().unwrap_or_else(|| {
            self.ident
                .as_ref()
                .map(|v| quote!(#v).to_string())
                .unwrap_or_default()
        });

        if self.flatten {
            quote!(&prefix)
        } else {
            quote!(::envstruct::concat_env_name(&prefix, #var_name))
        }
    }
}

impl ToTokens for EnvStructInputReceiver {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let EnvStructInputReceiver {
            ident,
            generics,
            data,
        } = self;
        let (imp, ty, where_clause) = generics.split_for_impl();

        let impl_block = match data {
            ast::Data::Enum(_) => {
                quote_spanned! {ty.span() =>
                    impl #imp ::envstruct::EnvParsePrimitive for #ident #ty #where_clause {
                        fn parse(val: &str) -> std::result::Result<Self, ::envstruct::StdError> {
                            Ok(val.parse::<#ident>()?)
                        }
                    }
                }
            }
            ast::Data::Struct(fields) => {
                let field_exprs: Vec<_> = fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        let field_name = field.name_exr(index);
                        let field_type = field.type_expr();
                        let var_default = field.default_expr();
                        let var_name_expr = field.var_name_expr();

                        if field.skip {
                             quote_spanned! {field.ty.span() =>
                                #field_name: Default::default()
                            }
                        } else {
                            quote_spanned! {field.ty.span() =>
                                #field_name: #field_type::parse_from_env_var(#var_name_expr, #var_default)?.into()
                            }
                        }

                    })
                    .collect();

                let inspect_exprs: Vec<_> = fields
                    .iter()
                    .filter(|field| !field.skip)
                    .map(|field| {
                        let field_type = field.type_expr();
                        let var_default = field.default_expr();
                        let var_name_expr = field.var_name_expr();

                        quote_spanned! {field.ty.span() =>
                            #field_type::get_env_entries(#var_name_expr, #var_default)?
                        }
                    })
                    .collect();

                quote! {
                    #[allow(clippy::useless_conversion)]
                    impl #imp ::envstruct::EnvParseNested for #ident #ty #where_clause {
                        fn parse_from_env_var(prefix: impl AsRef<str>, default: Option<&str>) -> std::result::Result<Self, ::envstruct::EnvStructError> {
                            Ok(Self {
                                #( #field_exprs, )*
                            })
                        }

                        fn get_env_entries(prefix: impl AsRef<str>, default: Option<&str>) -> std::result::Result<Vec<::envstruct::EnvEntry>, ::envstruct::EnvStructError> {
                            Ok(vec![#( #inspect_exprs, )*].into_iter().flatten().collect())
                        }
                    }
                }
            }
        };

        tokens.extend(impl_block);
    }
}
