use convert_case::{Case, Casing};
use fallible_iterator::FallibleIterator;
use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::{
    collections::{
        hash_map::{Entry, OccupiedEntry},
        HashMap,
    },
    env::args,
    iter::IntoIterator,
    slice::SliceIndex,
};
use syn::{
    parse_macro_input, Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Expr, ExprPath,
    Field, Fields, Ident, Lit, Meta, MetaList, MetaNameValue, NestedMeta, Path, PathSegment,
    Result,
};

mod example;

#[proc_macro_derive(Visitor, attributes(visitor))]
pub fn derive_visitor(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, impl_visitor)
}

#[proc_macro_derive(Walk, attributes(walk))]
pub fn derive_walk(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, impl_walk)
}

fn expand_with(
    input: proc_macro::TokenStream,
    handler: impl Fn(DeriveInput) -> Result<TokenStream>,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    handler(input)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

fn extract_attribute(attrs: Vec<Attribute>, attr_name: &str) -> Result<Option<MetaList>> {
    let attrs = attrs
        .into_iter()
        .filter(|attr| attr.path.is_ident(attr_name))
        .collect::<Vec<Attribute>>();

    if let Some(second) = attrs.get(2) {
        return Err(Error::new_spanned(second, "duplicate attribute"));
    }

    if let Some(attr) = attrs.first() {
        let meta = attr.parse_meta()?;
        if let Meta::List(meta_list) = meta {
            Ok(Some(meta_list))
        } else {
            Err(Error::new_spanned(attr, "invalid attribute"))
        }
    } else {
        Ok(None)
    }
}

fn extract_params(
    meta_list: &MetaList,
    allowed_params: Option<&[&str]>,
) -> Result<HashMap<Path, Meta>> {
    let params = HashMap::new();
    for nested in meta_list.nested {
        if let NestedMeta::Meta(meta) = nested {
            let path = meta.path();
            if let Some(ident) = meta.path().get_ident() {
                let param = ident.to_string();
                if let Some(allowed_params) = allowed_params {
                    if !allowed_params
                        .into_iter()
                        .any(|allowed_param| param == *allowed_param)
                    {
                        return Err(Error::new_spanned(ident, "unknown parameter"));
                    }
                }
                let entry = params.entry(path.clone());
                if matches!(entry, Entry::Occupied(_)) {
                    return Err(Error::new_spanned(ident, "duplicate parameter"));
                }
                entry.or_insert(meta);
            } else {
                return Err(Error::new_spanned(path, "invalid attribute"));
            }
        } else {
            return Err(Error::new_spanned(nested, "invalid attribute"));
        }
    }
    Ok(params)
}

type VisitorParams = HashMap<Path, VisitorItemParams>;

struct VisitorItemParams {
    enter: Option<Ident>,
    exit: Option<Ident>,
}

fn visitor_method_name(struct_path: &Path, op: &str) -> Ident {
    let last_segment = struct_path.segments.last().unwrap();
    Ident::new(
        &format!(
            "{}_{}",
            op,
            last_segment.ident.to_string().to_case(Case::Snake)
        ),
        Span::call_site(),
    )
}

fn impl_visitor(input: DeriveInput) -> Result<TokenStream> {
    let attibute = extract_attribute(input.attrs, "visitor")?;
    let params = if let Some(meta_list) = attibute {
        let params = fallible_iterator::convert(
            extract_params(&meta_list, None)?
                .iter()
                .map(|param| Ok(param)),
        )
        .map(|(path, meta)| match meta {
            Meta::List(meta_list) => {
                let item_params = extract_params(meta_list, Some(&["enter", "exit"]))?;
                fn extract_ident(
                    item_params: &HashMap<Path, Meta>,
                    name: &str,
                ) -> Result<Option<Ident>> {
                    item_params
                        .get(&Ident::new(name, Span::call_site()).into())
                        .map(|meta| match meta {
                            Meta::Path(path) => Ok(visitor_method_name(path, "enter")),
                            Meta::NameValue(name_value) => {
                                if let Lit::Str(str) = name_value.lit {
                                    Ok(str.parse()?)
                                } else {
                                    Err(Error::new_spanned(name_value.lit, "invalid attribute"))
                                }
                            }
                            _ => Err(Error::new_spanned(meta, "invalid attribute")),
                        })
                        .transpose()
                }
                Ok((
                    path,
                    VisitorItemParams {
                        enter: extract_ident(&item_params, "enter")?,
                        exit: extract_ident(&item_params, "exit")?,
                    },
                ))
            }
            Meta::Path(path) => Ok((
                path,
                VisitorItemParams {
                    enter: Some(visitor_method_name(path, "enter")),
                    exit: Some(visitor_method_name(path, "exit")),
                },
            )),
            _ => return Err(Error::new_spanned(meta, "invalid attribute parameter")),
        })
        .collect()?;
        params
    } else {
        HashMap::new()
    };

    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let routes = params
        .into_iter()
        .map(|(path, item_params)| visitor_route(path, item_params));
    Ok(quote! {
        impl #impl_generics ::derive_visitor::Visitor for #name #ty_generics #where_clause {
            fn drive(&mut self, item: &dyn ::std::any::Any, op: ::derive_visitor::Op) {
                #(
                    #routes
                )*
            }
        }
    })
}

fn visitor_route(path: &Path, item_params: VisitorItemParams) -> TokenStream {
    let enter_route = item_params.enter.map(visitor_method_call).into_iter();
    let exit_route = item_params.exit.map(visitor_method_call).into_iter();

    fn visitor_method_call(method: Ident) -> TokenStream {
        quote! {
            self.#method(item);
        }
    }

    quote! {
        if let Some(item) = <dyn ::std::any::Any>::downcast_ref::<#path>(item) {
            match op {
                ::derive_visitor::Op::Enter => { #( #enter_route )* },
                ::derive_visitor::Op::Exit => { #( #exit_route )* }
            }
        }
    }
}

fn impl_walk(input: DeriveInput) -> Result<TokenStream> {
    let attr = extract_attribute(input.attrs, "walk")?;
    let params = attr
        .map(|attr| extract_params(&attr, Some(&["skip"])))
        .transpose()?
        .unwrap_or_else(HashMap::new);
    let skip_visit_self = {
        match params.get(&Ident::new("skip", Span::call_site()).into()) {
            Some(Meta::Path(_)) => true,
            None => false,
            Some(meta) => return Err(Error::new_spanned(meta, "invalid attribute")),
        }
    };

    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let maybe_enter_self = if skip_visit_self {
        None
    } else {
        Some(quote! {
            ::derive_visitor::Visitor::drive(visitor, self, ::derive_visitor::Op::Enter);
        })
    };

    let maybe_exit_self = if skip_visit_self {
        None
    } else {
        Some(quote! {
            ::derive_visitor::Visitor::drive(visitor, self, ::derive_visitor::Op::Exit);
        })
    };

    let walk_fields = match input.data {
        Data::Struct(struct_) => walk_struct(struct_),
        Data::Enum(enum_) => walk_enum(enum_),
        Data::Union(union_) => {
            return Err(Error::new_spanned(
                union_.union_token,
                "unions are not supported",
            ))
        }
    };

    Ok(quote! {
        impl #impl_generics ::derive_visitor::Walk for #name #ty_generics #where_clause {
            fn walk<V: Visitor>(&self, visitor: &mut V) {
                #maybe_enter_self
                #maybe_exit_self
            }
        }
    })
}

fn walk_struct(struct_: DataStruct) -> Result<TokenStream> {
    Ok(struct_
        .fields
        .into_iter()
        .enumerate()
        .map(|(index, field)| {
            let path = field
                .ident
                .unwrap_or_else(|| Ident::new(&index.to_string(), Span::call_site()));
            walk_field(quote! { &self.#path }, field)
        })
        .collect::<Result<TokenStream>>()?)
}

fn walk_enum(enum_: DataEnum) -> Result<TokenStream> {
    let variants = enum_
        .variants
        .into_iter()
        .map(|variant| {
            let attr = extract_attribute(variant.attrs, "walk")?;
            let params = attr
                .map(|attr| extract_params(&attr, Some(&["skip"])))
                .transpose()?
                .unwrap_or_else(HashMap::new);
            match params.get(&Ident::new("skip", Span::call_site()).into()) {
                Some(Meta::Path(_)) => return Ok(quote! {}),
                None => {}
                Some(meta) => return Err(Error::new_spanned(meta, "invalid attribute")),
            }
            let name = variant.ident;
            let destructuring = {
                match variant.fields {
                    Fields::Named(fields) => {
                        let field_names =
                            fields.named.into_iter().map(|field| field.ident.unwrap());
                        quote! {
                            { #( #field_names ),* }
                        }
                    }
                    Fields::Unnamed(fields) => {
                        let field_names =
                            fields.unnamed.into_iter().enumerate().map(|(index, _)| {
                                Ident::new(&format!("i{}", index), Span::call_site())
                            });
                        quote! {
                            ( #( #field_names ),* )
                        }
                    }
                    Fields::Unit => return Ok(quote! {}),
                }
            };
            let fields = variant
                .fields
                .into_iter()
                .enumerate()
                .map(|(index, field)| {
                    walk_field(
                        quote! {field
                        .ident
                        .unwrap_or_else(|| {
                            Ident::new(&format!("i{}", index), Span::call_site())
                        })
                        .into() },
                        field,
                    )
                })
                .collect::<Result<TokenStream>>()?;
            Ok(quote! {
                Self::#name#destructuring => {
                    #fields
                }
            })
        })
        .collect::<Result<TokenStream>>()?;
    Ok(quote! {
        match self {
            #variants
            _ => {}
        }
    })
}

fn walk_field(expr: TokenStream, field: Field) -> Result<TokenStream> {
    let attr = extract_attribute(field.attrs, "walk")?;
    let params = attr
        .map(|attr| extract_params(&attr, Some(&["with", "skip"])))
        .transpose()?
        .unwrap_or_else(HashMap::new);

    match params.get(&Ident::new("skip", Span::call_site()).into()) {
        Some(Meta::Path(_)) => return Ok(quote! {}),
        None => {}
        Some(meta) => return Err(Error::new_spanned(meta, "invalid parameter")),
    }

    let walk_fn = params
        .get(&Ident::new("with", Span::call_site()).into())
        .map(|meta| {
            if let Meta::NameValue(MetaNameValue {
                lit: Lit::Str(str),
                eq_token: _,
                path: _,
            }) = meta
            {
                Ok(str.parse::<Path>()?)
            } else {
                Err(Error::new_spanned(meta, "invalid parameter"))
            }
        })
        .transpose()?
        .unwrap_or(Ident::new("::derive_visitor::Walk::walk", Span::call_site()).into());

    Ok(quote! {
        #walk_fn(#expr, visitor);
    })
}
