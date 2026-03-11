use darling::{ast::Style, FromDeriveInput, FromField, FromVariant};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{token::Mut, DeriveInput, Ident, Member, Path, Result};

#[derive(FromDeriveInput)]
#[darling(attributes(walk))]
struct WalkConfig {
    #[darling(default)]
    skip_self: bool,
    data: darling::ast::Data<WalkVariantConfig, WalkFieldConfig>,
}

#[derive(FromVariant)]
#[darling(attributes(walk))]
struct WalkVariantConfig {
    ident: Ident,
    #[darling(default)]
    skip: bool,
    fields: darling::ast::Fields<WalkFieldConfig>,
}

#[derive(FromField)]
#[darling(attributes(walk))]
struct WalkFieldConfig {
    ident: Option<Ident>,
    #[darling(default)]
    skip: bool,
    with: Option<Path>,
}

pub fn impl_walk(input: DeriveInput, mutable: bool) -> Result<TokenStream> {
    let config = WalkConfig::from_derive_input(&input)?;

    let skip_visit_self = config.skip_self;

    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let visitor = if mutable {
        quote! { VisitorMut }
    } else {
        quote! { Visitor }
    };

    let lifetime = if mutable {
        None
    } else {
        Some(quote! {
            'visitor,
        })
    };

    let bound_lifetime = if mutable {
        None
    } else {
        Some(quote! {
            <'visitor>
        })
    };

    let receiver_lifetime = if mutable {
        None
    } else {
        Some(quote! { 'visitor })
    };

    let enter_self = if skip_visit_self {
        None
    } else {
        Some(quote! {
            ::derive_visitor::#visitor::enter(visitor, self);
        })
    };

    let exit_self = if skip_visit_self {
        None
    } else {
        Some(quote! {
            ::derive_visitor::#visitor::exit(visitor, self);
        })
    };

    let walk_fields = match config.data {
        darling::ast::Data::Struct(fields) => walk_struct(&fields.fields, mutable),
        darling::ast::Data::Enum(variants) => walk_enum(&variants, mutable),
    };

    let impl_trait = Ident::new(if mutable { "WalkMut" } else { "Walk" }, Span::call_site());
    let method = Ident::new(if mutable { "walk_mut" } else { "walk" }, Span::call_site());
    let mut_modifier = if mutable {
        Some(Mut(Span::call_site()))
    } else {
        None
    };

    Ok(quote! {
        impl #impl_generics ::derive_visitor::#impl_trait for #name #ty_generics #where_clause {
            fn #method<#lifetime V: ::derive_visitor::#visitor #bound_lifetime + ?Sized>(& #receiver_lifetime #mut_modifier self, visitor: &mut V) {
                #enter_self
                #walk_fields
                #exit_self
            }
        }
    })
}

fn walk_struct(fields: &[WalkFieldConfig], mutable: bool) -> TokenStream {
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let member = field.ident.as_ref().map_or_else(
                || Member::Unnamed(index.into()),
                |ident| Member::Named(ident.clone()),
            );
            let mut_modifier = if mutable {
                Some(Mut(Span::call_site()))
            } else {
                None
            };
            walk_field(&quote! { & #mut_modifier self.#member }, field, mutable)
        })
        .collect()
}

fn walk_enum(variants: &[WalkVariantConfig], mutable: bool) -> TokenStream {
    let variants = variants
        .iter()
        .map(|x| walk_variant(x, mutable))
        .collect::<TokenStream>();
    quote! {
        match self {
            #variants
            _ => {}
        }
    }
}

fn walk_variant(variant: &WalkVariantConfig, mutable: bool) -> TokenStream {
    if variant.skip {
        return TokenStream::new();
    }
    let name = &variant.ident;
    let destructuring = destructure_fields(&variant.fields);
    let fields = variant
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            walk_field(
                &field
                    .ident
                    .clone()
                    .unwrap_or_else(|| Ident::new(&format!("i{index}"), Span::call_site()))
                    .to_token_stream(),
                field,
                mutable,
            )
        })
        .collect::<TokenStream>();
    quote! {
        Self::#name #destructuring => {
            #fields
        }
    }
}

fn destructure_fields(fields: &darling::ast::Fields<WalkFieldConfig>) -> TokenStream {
    match fields.style {
        Style::Struct => {
            let field_list = fields
                .fields
                .iter()
                .map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    if field.skip {
                        quote! { #field_name: _ }
                    } else {
                        field_name.into_token_stream()
                    }
                })
                .collect::<Vec<TokenStream>>();
            quote! {
                { #( #field_list ),* }
            }
        }
        Style::Tuple => {
            let field_list = fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    if field.skip {
                        quote! { _ }
                    } else {
                        Ident::new(&format!("i{index}"), Span::call_site()).into_token_stream()
                    }
                })
                .collect::<Vec<TokenStream>>();
            quote! {
                ( #( #field_list ),* )
            }
        }
        Style::Unit => TokenStream::new(),
    }
}

fn walk_field(value_expr: &TokenStream, field: &WalkFieldConfig, mutable: bool) -> TokenStream {
    if field.skip {
        return TokenStream::new();
    }

    let walk_fn = field.with.as_ref().map_or_else(
        || {
            if mutable {
                quote! { ::derive_visitor::WalkMut::walk_mut }
            } else {
                quote! { ::derive_visitor::Walk::walk }
            }
        },
        darling::ToTokens::into_token_stream,
    );

    quote! {
        #walk_fn(#value_expr, visitor);
    }
}
