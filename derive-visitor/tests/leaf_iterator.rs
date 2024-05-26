use std::any::Any;

use derive_visitor::{Event, ToLeafIter};

struct Example {
    heads: Heads,
    tails: Tails,
}

struct Heads;

impl ToLeafIter for Heads {
    fn to_leaf_iter(&self) -> impl derive_visitor::LeafIterator<'_> {
        ::std::iter::Iterator::chain(
            ::std::iter::once((self as &dyn std::any::Any, derive_visitor::Event::Enter)),
            ::std::iter::once((self as &dyn std::any::Any, derive_visitor::Event::Exit)),
        )
    }
}

struct Tails;

impl ToLeafIter for Tails {
    fn to_leaf_iter(&self) -> impl derive_visitor::LeafIterator<'_> {
        ::std::iter::Iterator::chain(
            ::std::iter::once((self as &dyn std::any::Any, derive_visitor::Event::Enter)),
            ::std::iter::once((self as &dyn std::any::Any, derive_visitor::Event::Exit)),
        )
    }
}

impl ToLeafIter for Example {
    fn to_leaf_iter(&self) -> impl derive_visitor::LeafIterator<'_> {
        ::std::iter::Iterator::chain(
            ::std::iter::Iterator::chain(
                ::std::iter::Iterator::chain(
                    ::std::iter::once((self as &dyn ::std::any::Any, derive_visitor::Event::Enter)),
                    self.heads.to_leaf_iter(),
                ),
                self.tails.to_leaf_iter(),
            ),
            ::std::iter::once((self as &dyn Any, Event::Exit)),
        )
    }
}
