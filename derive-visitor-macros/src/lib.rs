//! This is a utility crate for [derive-visitor](https://docs.rs/derive-visitor)
//!

#![warn(clippy::all)]
#![warn(clippy::pedantic)]

use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::{
    collections::{hash_map::Entry, HashMap},
    iter::IntoIterator,
};
use syn::token::Mut;
use syn::{
    parse_macro_input, parse_str, spanned::Spanned, Attribute, Data, DataEnum, DataStruct,
    DeriveInput, Error, Field, Fields, Ident, Lit, LitStr, Member, Meta, MetaList, NestedMeta,
    Path, Result, Variant,
};

#[proc_macro_derive(Visitor, attributes(visitor))]
pub fn derive_visitor(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |stream| impl_visitor(stream, false))
}

#[proc_macro_derive(VisitorMut, attributes(visitor))]
pub fn derive_visitor_mut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |stream| impl_visitor(stream, true))
}

#[proc_macro_derive(Drive, attributes(drive))]
pub fn derive_drive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |stream| impl_drive(stream, false))
}

#[proc_macro_derive(DriveMut, attributes(drive))]
pub fn derive_drive_mut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |stream| impl_drive(stream, true))
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

fn extract_meta(attrs: Vec<Attribute>, attr_name: &str) -> Result<Option<Meta>> {
    let macro_attrs = attrs
        .into_iter()
        .filter(|attr| attr.path.is_ident(attr_name))
        .collect::<Vec<Attribute>>();

    if let Some(second) = macro_attrs.get(2) {
        return Err(Error::new_spanned(second, "duplicate attribute"));
    }

    macro_attrs.first().map(Attribute::parse_meta).transpose()
}

#[derive(Default)]
struct Params(HashMap<Path, Meta>);

impl Params {
    fn from_attrs(attrs: Vec<Attribute>, attr_name: &str) -> Result<Self> {
        Ok(extract_meta(attrs, attr_name)?
            .map(|meta| {
                if let Meta::List(meta_list) = meta {
                    Self::from_meta_list(meta_list)
                } else {
                    Err(Error::new_spanned(meta, "invalid attribute"))
                }
            })
            .transpose()?
            .unwrap_or_default())
    }

    fn from_meta_list(meta_list: MetaList) -> Result<Self> {
        let mut params = HashMap::new();
        for meta in meta_list.nested {
            if let NestedMeta::Meta(meta) = meta {
                let path = meta.path();
                let entry = params.entry(path.clone());
                if matches!(entry, Entry::Occupied(_)) {
                    return Err(Error::new_spanned(path, "duplicate parameter"));
                }
                entry.or_insert(meta);
            } else {
                return Err(Error::new_spanned(meta, "invalid attribute"));
            }
        }
        Ok(Self(params))
    }

    fn validate(&self, allowed_params: &[&str]) -> Result<()> {
        for path in self.0.keys() {
            if !allowed_params
                .iter()
                .any(|allowed_param| path.is_ident(allowed_param))
            {
                return Err(Error::new_spanned(
                    path,
                    format!(
                        "unknown parameter, supported: {}",
                        Itertools::intersperse(allowed_params.iter().copied(), ", ")
                            .collect::<String>()
                    ),
                ));
            }
        }
        Ok(())
    }

    fn param(&mut self, name: &str) -> Result<Option<Param>> {
        self.0
            .remove(&Ident::new(name, Span::call_site()).into())
            .map(Param::from_meta)
            .transpose()
    }
}

impl Iterator for Params {
    type Item = Result<Param>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .keys()
            .next()
            .cloned()
            .map(|path| Param::from_meta(self.0.remove(&path).unwrap()))
    }
}

enum Param {
    Unit(Path, Span),
    StringLiteral(Path, Span, LitStr),
    NestedParams(Path, Span, Params),
}

impl Param {
    fn from_meta(meta: Meta) -> Result<Self> {
        let path = meta.path().clone();
        let span = meta.span();
        match meta {
            Meta::Path(_) => Ok(Param::Unit(path, span)),
            Meta::List(meta_list) => Ok(Param::NestedParams(
                path,
                span,
                Params::from_meta_list(meta_list)?,
            )),
            Meta::NameValue(name_value) => {
                if let Lit::Str(lit_str) = name_value.lit {
                    Ok(Param::StringLiteral(path, span, lit_str))
                } else {
                    Err(Error::new_spanned(name_value, "invalid parameter"))
                }
            }
        }
    }
    fn path(&self) -> &Path {
        match self {
            Self::Unit(path, _)
            | Self::StringLiteral(path, _, _)
            | Self::NestedParams(path, _, _) => path,
        }
    }

    fn span(&self) -> Span {
        match self {
            Self::Unit(_, span)
            | Self::StringLiteral(_, span, _)
            | Self::NestedParams(_, span, _) => *span,
        }
    }

    fn unit(self) -> Result<()> {
        if let Self::Unit(_, _) = self {
            Ok(())
        } else {
            Err(Error::new(self.span(), "invalid parameter"))
        }
    }

    fn string_literal(self) -> Result<LitStr> {
        if let Self::StringLiteral(_, _, lit_str) = self {
            Ok(lit_str)
        } else {
            Err(Error::new(self.span(), "invalid parameter"))
        }
    }
}

struct VisitorItemParams {
    enter: Option<Ident>,
    exit: Option<Ident>,
}

fn visitor_method_name_from_path(struct_path: &Path, event: &str) -> Ident {
    let last_segment = struct_path.segments.last().unwrap();
    Ident::new(
        &format!(
            "{}_{}",
            event,
            last_segment.ident.to_string().to_case(Case::Snake)
        ),
        Span::call_site(),
    )
}

fn visitor_method_name_from_param(param: Param, path: &Path, event: &str) -> Result<Ident> {
    match param {
        Param::StringLiteral(_, _, lit_str) => lit_str.parse(),
        Param::Unit(_, _) => Ok(visitor_method_name_from_path(path, event)),
        Param::NestedParams(_, span, _) => Err(Error::new(span, "invalid parameter")),
    }
}

fn impl_visitor(input: DeriveInput, mutable: bool) -> Result<TokenStream> {
    let params = Params::from_attrs(input.attrs, "visitor")?
        .map_ok(|param| {
            let path = param.path().clone();

            let item_params = match param {
                Param::Unit(_, _) => VisitorItemParams {
                    enter: Some(visitor_method_name_from_path(&path, "enter")),
                    exit: Some(visitor_method_name_from_path(&path, "exit")),
                },
                Param::NestedParams(_, _, mut nested) => {
                    nested.validate(&["enter", "exit"])?;
                    VisitorItemParams {
                        enter: nested
                            .param("enter")?
                            .map(|param| visitor_method_name_from_param(param, &path, "enter"))
                            .transpose()?,
                        exit: nested
                            .param("exit")?
                            .map(|param| visitor_method_name_from_param(param, &path, "exit"))
                            .transpose()?,
                    }
                }
                Param::StringLiteral(_, _, lit) => {
                    return Err(Error::new_spanned(lit, "invalid attribute"));
                }
            };
            Ok((path, item_params))
        })
        .flatten()
        .collect::<Result<HashMap<Path, VisitorItemParams>>>()?;

    match input.data {
        Data::Enum(enum_) => {
            for variant in enum_.variants {
                if let Some(attr) = variant.attrs.first() {
                    return Err(Error::new_spanned(
                        attr,
                        "#[visitor] attribute can only be applied to enum or struct",
                    ));
                }
                for field in variant.fields {
                    if let Some(attr) = field.attrs.first() {
                        return Err(Error::new_spanned(
                            attr,
                            "#[visitor] attribute can only be applied to enum or struct",
                        ));
                    }
                }
            }
        }
        Data::Struct(struct_) => {
            for field in struct_.fields {
                if let Some(attr) = field.attrs.first() {
                    return Err(Error::new_spanned(
                        attr,
                        "#[visitor] attribute can only be applied to enum or struct",
                    ));
                }
            }
        }
        Data::Union(union_) => {
            return Err(Error::new_spanned(
                union_.union_token,
                "unions are not supported",
            ));
        }
    }

    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let routes = params
        .into_iter()
        .map(|(path, item_params)| visitor_route(&path, item_params, mutable));
    let impl_trait = Ident::new(
        if mutable { "VisitorMut" } else { "Visitor" },
        Span::call_site(),
    );
    let mut_modifier = if mutable {
        Some(Mut(Span::call_site()))
    } else {
        None
    };
    Ok(quote! {
        impl #impl_generics ::derive_visitor::#impl_trait for #name #ty_generics #where_clause {
            fn visit(&mut self, item: & #mut_modifier dyn ::std::any::Any, event: ::derive_visitor::Event) {
                #(
                    #routes
                )*
            }
        }
    })
}

fn visitor_route(path: &Path, item_params: VisitorItemParams, mutable: bool) -> TokenStream {
    let enter = item_params.enter.map(|method_name| {
        quote! {
            ::derive_visitor::Event::Enter => {
                self.#method_name(item);
            }
        }
    });
    let exit = item_params.exit.map(|method_name| {
        quote! {
            ::derive_visitor::Event::Exit => {
                self.#method_name(item);
            }
        }
    });

    let method = Ident::new(
        if mutable {
            "downcast_mut"
        } else {
            "downcast_ref"
        },
        Span::call_site(),
    );

    quote! {
        if let Some(item) = <dyn ::std::any::Any>::#method::<#path>(item) {
            match event {
                #enter
                #exit
                _ => {}
            }
        }
    }
}

fn impl_drive(input: DeriveInput, mutable: bool) -> Result<TokenStream> {
    let mut params = Params::from_attrs(input.attrs, "drive")?;
    params.validate(&["skip"])?;

    let skip_visit_self = params
        .param("skip")?
        .map(Param::unit)
        .transpose()?
        .is_some();

    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let visitor = Ident::new(
        if mutable { "VisitorMut" } else { "Visitor" },
        Span::call_site(),
    );

    let enter_self = if skip_visit_self {
        None
    } else {
        Some(quote! {
            ::derive_visitor::#visitor::visit(visitor, self, ::derive_visitor::Event::Enter);
        })
    };

    let exit_self = if skip_visit_self {
        None
    } else {
        Some(quote! {
            ::derive_visitor::#visitor::visit(visitor, self, ::derive_visitor::Event::Exit);
        })
    };

    let drive_fields = match input.data {
        Data::Struct(struct_) => drive_struct(struct_, mutable),
        Data::Enum(enum_) => drive_enum(enum_, mutable),
        Data::Union(union_) => {
            return Err(Error::new_spanned(
                union_.union_token,
                "unions are not supported",
            ));
        }
    }?;

    let impl_trait = Ident::new(
        if mutable { "DriveMut" } else { "Drive" },
        Span::call_site(),
    );
    let method = Ident::new(
        if mutable { "drive_mut" } else { "drive" },
        Span::call_site(),
    );
    let mut_modifier = if mutable {
        Some(Mut(Span::call_site()))
    } else {
        None
    };

    Ok(quote! {
        impl #impl_generics ::derive_visitor::#impl_trait for #name #ty_generics #where_clause {
            fn #method<V: ::derive_visitor::#visitor>(& #mut_modifier self, visitor: &mut V) {
                #enter_self
                #drive_fields
                #exit_self
            }
        }
    })
}

fn drive_struct(struct_: DataStruct, mutable: bool) -> Result<TokenStream> {
    struct_
        .fields
        .into_iter()
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
            drive_field(&quote! { & #mut_modifier self.#member }, field, mutable)
        })
        .collect()
}

fn drive_enum(enum_: DataEnum, mutable: bool) -> Result<TokenStream> {
    let variants = enum_
        .variants
        .into_iter()
        .map(|x| drive_variant(x, mutable))
        .collect::<Result<TokenStream>>()?;
    Ok(quote! {
        match self {
            #variants
            _ => {}
        }
    })
}

fn drive_variant(variant: Variant, mutable: bool) -> Result<TokenStream> {
    let mut params = Params::from_attrs(variant.attrs, "drive")?;
    params.validate(&["skip"])?;
    if params.param("skip")?.map(Param::unit).is_some() {
        return Ok(TokenStream::new());
    }
    let name = variant.ident;
    let destructuring = destructure_fields(variant.fields.clone())?;
    let fields = variant
        .fields
        .into_iter()
        .enumerate()
        .map(|(index, field)| {
            drive_field(
                &field
                    .ident
                    .clone()
                    .unwrap_or_else(|| Ident::new(&format!("i{}", index), Span::call_site()))
                    .to_token_stream(),
                field,
                mutable,
            )
        })
        .collect::<Result<TokenStream>>()?;
    Ok(quote! {
        Self::#name#destructuring => {
            #fields
        }
    })
}

fn destructure_fields(fields: Fields) -> Result<TokenStream> {
    Ok(match fields {
        Fields::Named(fields) => {
            let field_list = fields
                .named
                .into_iter()
                .map(|field| {
                    let mut params = Params::from_attrs(field.attrs, "drive")?;
                    let field_name = field.ident.unwrap();
                    Ok(if params.param("skip")?.map(Param::unit).is_some() {
                        quote! { #field_name: _ }
                    } else {
                        field_name.into_token_stream()
                    })
                })
                .collect::<Result<Vec<TokenStream>>>()?;
            quote! {
                { #( #field_list ),* }
            }
        }
        Fields::Unnamed(fields) => {
            let field_list = fields
                .unnamed
                .into_iter()
                .enumerate()
                .map(|(index, field)| {
                    let mut params = Params::from_attrs(field.attrs, "drive")?;
                    Ok(if params.param("skip")?.map(Param::unit).is_some() {
                        quote! { _ }
                    } else {
                        Ident::new(&format!("i{}", index), Span::call_site()).into_token_stream()
                    })
                })
                .collect::<Result<Vec<TokenStream>>>()?;
            quote! {
                ( #( #field_list ),* )
            }
        }
        Fields::Unit => TokenStream::new(),
    })
}

fn drive_field(value_expr: &TokenStream, field: Field, mutable: bool) -> Result<TokenStream> {
    let mut params = Params::from_attrs(field.attrs, "drive")?;
    params.validate(&["skip", "with"])?;

    if params.param("skip")?.map(Param::unit).is_some() {
        return Ok(TokenStream::new());
    }

    let drive_fn = params.param("with")?.map_or_else(
        || {
            parse_str(if mutable {
                "::derive_visitor::DriveMut::drive_mut"
            } else {
                "::derive_visitor::Drive::drive"
            })
        },
        |param| param.string_literal()?.parse::<Path>(),
    )?;

    Ok(quote! {
        #drive_fn(#value_expr, visitor);
    })
}
