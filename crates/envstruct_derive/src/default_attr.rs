/// Represents the default value for a field in the `EnvStruct`.
#[derive(Debug)]
pub enum DefaultAttr {
    /// A string literal default value.
    String(String),
    /// A type that provides a default value.
    Type(syn::Type),
    /// The default value provided by the type's `Default` implementation.
    Default,
}

impl darling::FromMeta for DefaultAttr {
    /// Parses a default value from a word.
    fn from_word() -> darling::Result<Self> {
        Ok(DefaultAttr::Default)
    }

    /// Parses a default value from a string.
    fn from_string(value: &str) -> darling::Result<Self> {
        Ok(DefaultAttr::String(value.to_string()))
    }

    /// Parses a default value from a boolean.
    fn from_bool(value: bool) -> darling::Result<Self> {
        Ok(DefaultAttr::String(value.to_string()))
    }

    /// Parses a default value from a literal.
    fn from_value(value: &syn::Lit) -> darling::Result<Self> {
        match value {
            syn::Lit::Str(lit_str) => Ok(DefaultAttr::String(lit_str.value())),
            syn::Lit::Int(lit_int) => Ok(DefaultAttr::String(lit_int.to_string())),
            syn::Lit::Bool(lit_bool) => Ok(DefaultAttr::String(lit_bool.value().to_string())),
            _ => Err(darling::Error::unexpected_lit_type(value)),
        }
    }

    /// Parses a default value from an expression.
    fn from_expr(expr: &syn::Expr) -> darling::Result<Self> {
        match expr {
            syn::Expr::Lit(expr_lit) => Self::from_value(&expr_lit.lit),
            syn::Expr::Path(expr_path) => {
                let ty: syn::Type = syn::parse_quote!(#expr_path);
                Ok(DefaultAttr::Type(ty))
            }
            _ => Err(darling::Error::unsupported_format(
                "Expected a string literal or type",
            )),
        }
    }
}
