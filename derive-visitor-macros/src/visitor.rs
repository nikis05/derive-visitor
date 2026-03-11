use darling::{util::Override, FromDeriveInput, FromMeta};
use heck::ToSnakeCase;
use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::{token::Mut, DeriveInput, Ident, Lifetime, LifetimeParam, Path, Result};

#[derive(FromDeriveInput)]
#[darling(attributes(visitor))]
struct VisitorConfig {
    ident: Ident,
    #[darling(flatten)]
    routes: HashMap<Path, VisitorItemConfig>,
}

#[derive(FromMeta)]
#[darling(from_word = || Ok(Default::default()))]
struct VisitorItemConfig {
    enter: Option<Override<String>>,
    exit: Option<Override<String>>,
}

impl Default for VisitorItemConfig {
    fn default() -> Self {
        Self {
            enter: Some(Override::Inherit),
            exit: Some(Override::Inherit),
        }
    }
}

pub fn impl_visitor(mut input: DeriveInput, mutable: bool) -> Result<TokenStream> {
    let config = VisitorConfig::from_derive_input(&input)?;
    let name = config.ident;

    let split_for_impl = input.generics.split_for_impl();
    let mut impl_generics = split_for_impl.0.into_token_stream();
    let ty_generics = split_for_impl.1.to_token_stream();
    let where_clause = split_for_impl.2.to_token_stream();

    if !mutable
        && !input
            .generics
            .lifetimes()
            .any(|lifetime_param| lifetime_param.lifetime.ident == "visitor")
    {
        input
            .generics
            .params
            .push(syn::GenericParam::Lifetime(LifetimeParam::new(
                Lifetime::new("'visitor", Span::call_site()),
            )));

        impl_generics = input.generics.split_for_impl().0.to_token_stream();
    }

    let enter_routes = config.routes.iter().filter_map(|(path, item_config)| {
        item_config
            .enter
            .as_ref()
            .map(|enter_config| visitor_route(path, enter_config, mutable, false))
    });

    let exit_routes = config.routes.iter().filter_map(|(path, item_config)| {
        item_config
            .exit
            .as_ref()
            .map(|enter_config| visitor_route(path, enter_config, mutable, true))
    });

    let impl_trait = Ident::new(
        if mutable { "VisitorMut" } else { "Visitor" },
        Span::call_site(),
    );

    let mut_modifier = if mutable {
        Some(Mut(Span::call_site()))
    } else {
        None
    };

    let trait_lifetime = if mutable {
        None
    } else {
        Some(quote! { <'visitor> })
    };

    Ok(quote! {
        impl #impl_generics ::derive_visitor::#impl_trait #trait_lifetime for #name #ty_generics #where_clause {
            fn enter(&mut self, item: & #mut_modifier dyn ::std::any::Any) {
                #(
                    #enter_routes
                )*
            }

            fn exit(&mut self, item: & #mut_modifier dyn ::std::any::Any) {
                #(
                    #exit_routes
                )*
            }
        }
    })
}

fn visitor_route(
    path: &Path,
    route_config: &Override<String>,
    mutable: bool,
    exit: bool,
) -> TokenStream {
    let method_name = Ident::new(
        &route_config.clone().unwrap_or_else(|| {
            format!("{}_{}", if exit { "exit" } else { "enter" }, {
                path.segments
                    .iter()
                    .map(|segment| segment.ident.to_string().to_snake_case())
                    .join("_")
            })
        }),
        Span::call_site(),
    );

    let downcast_method = Ident::new(
        if mutable {
            "downcast_mut"
        } else {
            "downcast_ref"
        },
        Span::call_site(),
    );

    quote! {
        if let Some(item) = <dyn ::std::any::Any>::#downcast_method::<#path>(item) {
            self.#method_name(item);
        }
    }
}
