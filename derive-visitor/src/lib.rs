#![warn(clippy::all)]
#![warn(clippy::pedantic)]

//! This crate derives [visitor pattern](https://rust-unofficial.github.io/patterns/patterns/behavioural/visitor.html)
//! for arbitrary data structures. This pattern is particularly useful when dealing with complex nested data structures,
//! abstract trees and hierarchies of all kinds.
//!
//! The main building blocks of this crate are two derivable traits:
//! - [Visitor] implementations walk through data structures and accumulates some information;
//! - [Drive] implementations are data structures that know how to drive a visitor through themselves.
//!
//! Please refer to these traits' documentation for more details.
//!
//! ## Example
//!
//! ```
//! use derive_visitor::{Visitor, Drive};
//!
//! #[derive(Drive)]
//! struct Directory {
//!     #[drive(skip)]
//!     name: String,
//!     items: Vec<DirectoryItem>,
//! }
//!
//! #[derive(Drive)]
//! enum DirectoryItem {
//!     File(File),
//!     Directory(Directory),
//! }
//!
//! #[derive(Drive)]
//! struct File {
//!     #[drive(skip)]
//!     name: String,
//! }
//!
//! #[derive(Visitor, Default)]
//! #[visitor(File(enter), Directory(enter))]
//! struct Counter {
//!     files: u32,
//!     directories: u32
//! }
//!
//! impl Counter {
//!     fn enter_file(&mut self, _file: &File) {
//!         self.files += 1;
//!     }
//!     fn enter_directory(&mut self, _directory: &Directory) {
//!         self.directories += 1;
//!     }
//! }
//!
//! let mut counter = Counter::default();
//!
//! let example_directory = Directory {
//!     name: "root".into(),
//!     items: vec![
//!         DirectoryItem::Directory(
//!             Directory {
//!                 name: "home".into(),
//!                 items: vec![
//!                     DirectoryItem::File(File { name: "README.md".into() }),
//!                     DirectoryItem::File(File { name: "Star Wars.mov".into() })
//!                 ]
//!             }
//!         ),
//!         DirectoryItem::Directory(
//!             Directory { name: "downloads".into(), items: vec![] }
//!         )
//!     ],
//! };
//!
//! example_directory.drive(&mut counter);
//!
//! assert_eq!(counter.files, 2);
//! assert_eq!(counter.directories, 3);
//! ```

/// See [Drive].
pub use derive_visitor_macros::Drive;

/// See [Visitor].
pub use derive_visitor_macros::Visitor;

use std::marker::PhantomData;
use std::{
    any::Any,
    cell::Cell,
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque},
};

/// An interface for visiting arbitrary data structures.
///
/// A visitor receives items that implement [Any], and can use dynamic dispatch
/// to downcast them to particular types that it is interested in. In the classical
/// visitor pattern, a Visitor has a set of separate methods to deal with each particular
/// item type. This behavior can be implemented automatically using derive.
///
/// ## Derivable
///
/// This trait can be derived for any struct or enum. By default, the derived implementation
/// does nothing. You need to explicitly specify what item types and / or events your visitor
/// is interested in, using top-level attribute:
///
/// ```ignore
/// #[derive(Visitor)]
/// #[visitor(Directory, File)]
/// struct NameValidator {
///     errors: Vec<InvalidNameError>,
/// }
///
/// impl NameValidator {
///     fn enter_directory(&mut self, item: &Directory) {
///         // ...your logic here
///     }
///     fn exit_directory(&mut self, item: &Directory) {
///         // ...your logic here
///     }
///     fn enter_file(&mut self, item: &File) {
///         // ...your logic here
///     }
///     fn exit_file(&mut self, item: &File) {
///         // ...your logic here
///     }
/// }
/// ```
///
/// ## Visitor functions / closures
/// If your visitor is only interested in some particular type, you don't have to declare a struct,
/// you can just create a visitor from a closure or a function, e.g.:
///
/// ```ignore
/// let file_visitor = visitor_fn(|file: &File, event| {
///     // ...your logic here
/// });
/// ```
///
/// See [visitor_fn](visitor_fn) and [visitor_enter_fn](visitor_enter_fn) for more info.
///
/// ## Macro attributes
///
/// If your visitor is only interested in [Event::Enter](Event::Enter) or [Event::Exit](Event::Exit),
/// you can configure the derived implementation to only call enter / exit, respectively,
/// on a per-type basis:
///
/// ```ignore
/// #[derive(Visitor)]
/// #[visitor(Directory(enter), File(exit))]
/// struct NameValidator {
///     errors: Vec<InvalidNameError>,
/// }
///
/// impl NameValidator {
///     fn enter_directory(&mut self, item: &Directory) {
///         // ...your logic here
///     }
///     fn exit_file(&mut self, item: &File) {
///         // ...your logic here
///     }
/// }
/// ```
///
/// You can also provide custom method names for each type / event:
///
/// ```ignore
/// #[derive(Visitor)]
/// #[visitor(Directory(enter="custom_enter_directory", exit="custom_exit_directory"), File)]
/// struct NameValidator {
///     errors: Vec<InvalidNameError>,
/// }
///
/// impl NameValidator {
///     fn custom_enter_directory(&mut self, item: &Directory) {
///         // ...your logic here
///     }
///     fn custom_exit_directory(&mut self, item: &Directory) {
///         // ...your logic here
///     }
///     fn enter_file(&mut self, item: &File) {
///         // ...your logic here
///     }
///     fn exit_file(&mut self, item: &File) {
///         // ...your logic here
///     }
/// }
/// ```
pub trait Visitor {
    fn visit(&mut self, item: &dyn Any, event: Event);
}

/// Create a visitor that only visits items of some specific type from a function or a closure.
///
/// ## Example
/// ```ignore
/// let file_visitor = visitor_fn(|file: &File, event| {
///     // ...your logic here
/// });
/// ```
pub fn visitor_fn<T: Any, F: FnMut(&T, Event)>(fun: F) -> FnVisitor<T, F> {
    FnVisitor {
        _marker: PhantomData,
        fun,
    }
}

/// Similar to [visitor_fn](visitor_fn), but the closure will only be called on [Event::Enter](Event::Enter).
pub fn visitor_enter_fn<T: Any, F: FnMut(&T)>(fun: F) -> EnterFnVisitor<T, F> {
    EnterFnVisitor {
        _marker: PhantomData,
        fun,
    }
}

/// Type returned by [visitor_fn].
pub struct FnVisitor<T: Any, F: FnMut(&T, Event)> {
    _marker: PhantomData<*const T>,
    fun: F,
}

impl<T: Any, F: FnMut(&T, Event)> Visitor for FnVisitor<T, F> {
    fn visit(&mut self, item: &dyn Any, event: Event) {
        if let Some(item) = <dyn Any>::downcast_ref::<T>(item) {
            let fun = &mut self.fun;
            fun(item, event);
        }
    }
}

/// Type returned by [visitor_enter_fn].
pub struct EnterFnVisitor<T: Any, F: FnMut(&T)> {
    _marker: PhantomData<*const T>,
    fun: F,
}

impl<T: Any, F: FnMut(&T)> Visitor for EnterFnVisitor<T, F> {
    fn visit(&mut self, item: &dyn Any, event: Event) {
        if let (Event::Enter, Some(item)) = (event, <dyn Any>::downcast_ref::<T>(item)) {
            let fun = &mut self.fun;
            fun(item);
        }
    }
}

/// Defines whether an item is being entered or exited by a visitor.
pub enum Event {
    Enter,
    Exit,
}

/// A data structure that can drive a [visitor](Visitor) through iself.
///
/// Derive or implement this trait for any type that you want to be able to
/// traverse with a visitor.
///
/// `Drive` is implemented for most wrapping and collection types from [std],
/// as long as their wrapped / item type implements `Drive`.
///
/// ## Derivable
///
/// This trait can be derived for any struct or enum.
/// By default, the derived implementation will make the visitor enter `self`,
/// then drive it through every field of `self`, and finally make it exit `self`:
///
/// ```ignore
/// #[derive(Drive)]
/// struct Directory {
///     #[drive(skip)]
///     name: String,
///     items: Vec<DirectoryItem>,
/// }
///
/// #[derive(Drive)]
/// enum DirectoryItem {
///     File(File),
///     Directory(Directory),
/// }
///
/// #[derive(Drive)]
/// struct File {
///     #[drive(skip)]
///     name: String,
/// }
/// ```
///
/// ## Implementing manually
///
/// The following code snippet is roughly equivalent to the implementations
/// that would be derived in the example above:
///
/// ```ignore
/// impl Drive for Directory {
///     fn drive<V: Visitor>(&self, visitor: &mut V) {
///         visitor.visit(self, Event::Enter);
///         self.items.drive(visitor);
///         visitor.visit(self, Event::Exit);
///     }
/// }
///
/// impl Drive for DirectoryItem {
///     fn drive<V: Visitor>(&self, visitor: &mut V) {
///         visitor.visit(self, Event::Enter);
///         match self {
///             Self::File(file) => {
///                 file.drive(visitor);
///             },
///             Self::Directory(directory) => {
///                 directory.drive(visitor);
///             }
///         }
///         visitor.visit(self, Event::Exit);
///     }
/// }
///
/// impl Drive for File {
///     fn drive<V: Visitor>(&self, visitor: &mut V) {
///         visitor.visit(self, Event::Enter);
///         visitor.visit(self, Event::Exit);
///     }
/// }
/// ```
///
/// ## Macro attributes
///
/// The derived implementation of `Drive` can be customized using attributes:
///
/// ### `#[drive(skip)]`
///
/// If applied to a field or an enum variant, the derived implementation won't
/// drive the visitor through that field / variant.
///
/// If applied to a struct or an enum itself, the derived implementation will
/// drive the visitor through the type's fields / variants, but won't make it
/// enter or exit the type itself.
///
/// ### `#[drive(with="path")]`
///
/// Drive a visitor through a field using a custom function.
/// The function must have the following signature: `fn<V: Visitor>(&T, &mut V)`.
///
/// In the example below, this attribute is used to customize driving through a [Vec]:
///
/// ```ignore
/// #[derive(Drive)]
/// struct Book {
///     title: String,
///     #[drive(with="reverse_vec_driver")]
///     chapters: Vec<Chapter>,
/// }
///
/// fn reverse_vec_driver<T, V: Visitor>(vec: &Vec<T>, visitor: &mut V) {
///     for item in vec.iter().rev() {
///         item.drive(visitor);
///     }
/// }
/// ```
pub trait Drive: Any {
    fn drive<V: Visitor>(&self, visitor: &mut V);
}

impl<K, Val> Drive for BTreeMap<K, Val>
where
    K: Drive,
    Val: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        for (key, value) in self.iter() {
            key.drive(visitor);
            value.drive(visitor);
        }
    }
}

impl<T> Drive for BTreeSet<T>
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.drive(visitor));
    }
}

impl<T> Drive for BinaryHeap<T>
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.drive(visitor));
    }
}

impl<T> Drive for Box<T>
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        (**self).drive(visitor);
    }
}

impl<T> Drive for Cell<T>
where
    T: Drive + Copy,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        self.get().drive(visitor);
    }
}

impl<K, Val, S> Drive for HashMap<K, Val, S>
where
    K: Drive,
    Val: Drive,
    S: 'static,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        for (key, value) in self.iter() {
            key.drive(visitor);
            value.drive(visitor);
        }
    }
}

impl<T, S> Drive for HashSet<T, S>
where
    T: Drive,
    S: 'static,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.drive(visitor));
    }
}

impl<T> Drive for LinkedList<T>
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.drive(visitor));
    }
}

impl<T> Drive for Option<T>
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        if let Some(value) = self {
            value.drive(visitor);
        }
    }
}

impl<T> Drive for Vec<T>
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.drive(visitor));
    }
}

impl<T> Drive for VecDeque<T>
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.drive(visitor));
    }
}

impl Drive for () {
    fn drive<V: Visitor>(&self, _visitor: &mut V) {}
}

macro_rules! tuple_impls {
    ( $( $( $type:ident ),+ => $( $field:tt ),+ )+ ) => {
        $(
            impl<$( $type ),+> Drive for ($($type,)+)
            where
                $(
                    $type: Drive
                ),+
            {
                fn drive<V: Visitor>(&self, visitor: &mut V) {
                    $(
                        self.$field.drive(visitor);
                    )+
                }
            }
        )+
    };
}

tuple_impls! {
    T0 => 0
    T0, T1 => 0, 1
    T0, T1, T2 => 0, 1, 2
    T0, T1, T2, T3 => 0, 1, 2, 3
    T0, T1, T2, T3, T4 => 0, 1, 2, 3, 4
    T0, T1, T2, T3, T4, T5 => 0, 1, 2, 3, 4, 5
    T0, T1, T2, T3, T4, T5, T6 => 0, 1, 2, 3, 4, 5, 6
    T0, T1, T2, T3, T4, T5, T6, T7 => 0, 1, 2, 3, 4, 5, 6, 7
}

impl<T> Drive for [T; 0]
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, _visitor: &mut V) {}
}

macro_rules! array_impls {
    ( $( $len:expr => $( $field:expr ),+ )+ ) => {
        $(
            impl<T> Drive for [T; $len]
            where
                T: Drive
            {
                fn drive<V: Visitor>(&self, visitor: &mut V) {
                    $(
                        self[$field].drive(visitor);
                    )+
                }
            }
        )+
    };
}

array_impls! {
    1 => 0
    2 => 0, 1
    3 => 0, 1, 2
    4 => 0, 1, 2, 3
    5 => 0, 1, 2, 3, 4
    6 => 0, 1, 2, 3, 4, 5
    7 => 0, 1, 2, 3, 4, 5, 6
    8 => 0, 1, 2, 3, 4, 5, 6, 7
}
