//! This is a utility crate for [derive-visitor](https://docs.rs/derive-visitor)
//!

#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::needless_continue)]

use proc_macro2::TokenStream;
use syn::{parse_macro_input, DeriveInput, Result};

#[proc_macro_derive(Visitor, attributes(visitor))]
pub fn derive_visitor(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |stream| visitor::impl_visitor(stream, false))
}

#[proc_macro_derive(VisitorMut, attributes(visitor))]
pub fn derive_visitor_mut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |stream| visitor::impl_visitor(stream, true))
}

#[proc_macro_derive(Walk, attributes(walk))]
pub fn derive_walk(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |stream| walk::impl_walk(stream, false))
}

#[proc_macro_derive(WalkMut, attributes(walk))]
pub fn derive_walk_mut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |stream| walk::impl_walk(stream, true))
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

mod visitor;
mod walk;
