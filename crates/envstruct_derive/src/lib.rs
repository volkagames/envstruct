mod default_attr;
mod normalize_type_path;

use darling::{ast, FromDeriveInput, FromField, FromVariant};
use default_attr::*;
use normalize_type_path::*;
use proc_macro::TokenStream;
use quote::*;
use syn::spanned::Spanned;

/// Derives the `EnvStruct` trait for a struct or enum.
#[proc_macro_derive(EnvStruct, attributes(env))]
pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input: syn::DeriveInput = syn::parse(input).expect("Failed to parse derive input");
    let receiver = EnvStructInputReceiver::from_derive_input(&derive_input)
        .expect("Failed to parse input for darling receiver");
    quote!(#receiver).into()
}

/// Receiver for the `EnvStruct` derive input.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(env), supports(any))]
struct EnvStructInputReceiver {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<EnvStructVariantReceiver, EnvStructFieldReceiver>,
    title: Option<String>,
}

/// Receiver for enum variants of the `EnvStruct`.
#[derive(Debug, FromVariant)]
struct EnvStructVariantReceiver {
    ident: syn::Ident,
}

/// Receiver for the fields of the `EnvStruct`.
#[derive(Debug, FromField)]
#[darling(attributes(env))]
struct EnvStructFieldReceiver {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    name: Option<String>,
    default: Option<DefaultAttr>,
    with: Option<syn::Expr>,
    title: Option<String>,
    used_if: Option<String>,
    #[darling(default)]
    flatten: bool,
    #[darling(default)]
    inline: bool,
    #[darling(default)]
    skip: bool,
}

impl EnvStructFieldReceiver {
    /// Generates a token stream for the field name or index.
    pub fn name_exr(&self, index: usize) -> proc_macro2::TokenStream {
        self.ident
            .as_ref()
            .map(quote::ToTokens::to_token_stream)
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(#index)
            })
    }

    /// Generates a token stream for the field type.
    pub fn type_expr(&self) -> proc_macro2::TokenStream {
        self.with
            .as_ref()
            .map(|ty| quote_spanned! { ty.span() => #ty })
            .unwrap_or({
                let ty = normalize_type_path(&self.ty);
                quote_spanned! { ty.span() => #ty }
            })
    }

    /// Generates a token stream for the default value of the field.
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

    /// Generates a token stream for the environment variable name.
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

    fn field_name_str(&self) -> String {
        self.ident
            .as_ref()
            .map(|ident| ident.to_string())
            .unwrap_or_default()
    }
}

fn parse_used_if(spec: &str) -> Result<(String, String), String> {
    let Some((field, value)) = spec.split_once('=') else {
        return Err(format!("used_if must be `field=value`, got `{spec}`"));
    };
    if field.is_empty() || value.is_empty() {
        return Err(format!("used_if must be `field=value`, got `{spec}`"));
    }
    Ok((field.to_string(), value.to_string()))
}

fn used_if_expr(
    field: &EnvStructFieldReceiver,
    fields: &ast::Fields<EnvStructFieldReceiver>,
) -> proc_macro2::TokenStream {
    let Some(spec) = &field.used_if else {
        return quote!(None);
    };
    let (sibling_name, value) = match parse_used_if(spec) {
        Ok(parsed) => parsed,
        Err(err) => {
            let ty = &field.ty;
            return quote_spanned! { ty.span() => compile_error!(#err) };
        }
    };
    let Some(sibling) = fields
        .iter()
        .find(|item| item.field_name_str() == sibling_name)
    else {
        let ty = &field.ty;
        let err = format!("used_if references unknown field `{sibling_name}`");
        return quote_spanned! { ty.span() => compile_error!(#err) };
    };
    let sibling_var_name = sibling.var_name_expr();
    let sibling_default = sibling.default_expr();
    quote! {
        Some(::envstruct::UsageUsedIf {
            env_name: #sibling_var_name,
            value: #value.to_string(),
            switch_default: #sibling_default.map(|value| value.to_string()),
        })
    }
}

impl ToTokens for EnvStructInputReceiver {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let EnvStructInputReceiver {
            ident,
            generics,
            data,
            title,
        } = self;
        let (imp, ty, where_clause) = generics.split_for_impl();

        let impl_block = match data {
            ast::Data::Enum(variants) => {
                let variant_names: Vec<String> = variants
                    .iter()
                    .map(|variant| variant.ident.to_string())
                    .collect();
                quote_spanned! {ty.span() =>
                    impl #imp ::envstruct::EnvParsePrimitive for #ident #ty #where_clause {
                        fn parse(val: &str) -> std::result::Result<Self, ::envstruct::BoxError> {
                            Ok(val.parse::<#ident>()?)
                        }

                        fn usage_type() -> ::envstruct::UsageType {
                            ::envstruct::UsageType::Enum
                        }

                        fn usage_values() -> Option<Vec<String>> {
                            Some(vec![#( #variant_names.to_string(), )*])
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
                        let field_name_str = field.field_name_str();
                        let flatten = field.flatten;
                        let inline = field.inline;
                        let title_expr = match &field.title {
                            Some(title) => quote!(Some(#title.to_string())),
                            None => quote!(None),
                        };
                        let used_if = used_if_expr(field, fields);
                        quote_spanned! {field.ty.span() =>
                            ::envstruct::attach_field_usage(
                                #field_type::get_usage_tree(#var_name_expr, #var_default)?,
                                ::envstruct::FieldUsageMeta {
                                    field_name: #field_name_str,
                                    title: #title_expr,
                                    flatten: #flatten,
                                    inline: #inline,
                                    used_if: #used_if,
                                },
                            )
                        }
                    })
                    .collect();

                let struct_title = match title {
                    Some(title) => quote!(Some(#title.to_string())),
                    None => quote!(None),
                };

                quote! {
                    #[allow(clippy::useless_conversion)]
                    impl #imp ::envstruct::EnvParseNested for #ident #ty #where_clause {
                        fn parse_from_env_var(prefix: impl AsRef<str>, default: Option<&str>) -> std::result::Result<Self, ::envstruct::EnvStructError> {
                            let _ = default;
                            Ok(Self {
                                #( #field_exprs, )*
                            })
                        }

                        fn get_usage_tree(prefix: impl AsRef<str>, default: Option<&str>) -> std::result::Result<::envstruct::UsageTree, ::envstruct::EnvStructError> {
                            let _ = default;
                            let nested: Vec<Vec<::envstruct::UsageItem>> = vec![#( #inspect_exprs, )*];
                            Ok(::envstruct::UsageTree {
                                title: #struct_title,
                                kind: ::envstruct::UsageTreeKind::Struct,
                                items: nested.into_iter().flatten().collect(),
                            })
                        }
                    }
                }
            }
        };

        tokens.extend(impl_block);
    }
}
