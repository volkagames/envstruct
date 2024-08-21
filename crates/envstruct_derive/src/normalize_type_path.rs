// transform Option<T> to Option::<T>
pub fn normalize_type_path(ty: &syn::Type) -> syn::Type {
    let mut ty = ty.clone();
    if let syn::Type::Path(type_path) = &mut ty {
        for segment in &mut type_path.path.segments {
            if let syn::PathArguments::AngleBracketed(ref mut args) = &mut segment.arguments {
                if args.colon2_token.is_none() {
                    args.colon2_token = Some(syn::Token!(::)([
                        proc_macro2::Span::call_site(),
                        proc_macro2::Span::call_site(),
                    ]));
                }
            }
        }
    }
    ty
}
