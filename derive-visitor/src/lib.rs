#![warn(clippy::all)]
#![warn(clippy::pedantic)]

//! This crate derives [visitor pattern](https://rust-unofficial.github.io/patterns/patterns/behavioural/visitor.html)
//! for arbitrary data structures. This pattern is particularly useful when dealing with complex nested data structures,
//! abstract trees and hierarchies of all kinds.
//!
//! The main building blocks of this crate are two derivable traits:
//! - [`Visitor`] and [`VisitorMut`] implementations walk through data structures and accumulate some information;
//! - [`Drive`] and [`DriveMut`] implementations are data structures that know how to drive a visitor through themselves.
//!
//! Please refer to these traits' documentation for more details.
//!
//! ## Example
//!
//! ### Immutable visitor that counts items in a tree
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
//!
//! ### Mutable visitor that alters a tree
//!
//! ```rust
//! use derive_visitor::{VisitorMut, DriveMut};
//!
//! #[derive(DriveMut)]
//! struct Tree {
//!     #[drive(skip)]
//!     name: String,
//!     children: Vec<Tree>
//! }
//!
//! #[derive(VisitorMut, Default)]
//! #[visitor(Tree(enter))]
//! struct Renamer {
//!     from: &'static str,
//!     to: &'static str,
//! }
//!
//! impl Renamer {
//!     fn enter_tree(&mut self, tree: &mut Tree) {
//!         tree.name = tree.name.replace(self.from, self.to);
//!     }
//! }
//!
//! let mut my_tree = Tree{
//!     name: "old parent".to_string(),
//!     children: vec![
//!         Tree {
//!             name: "old child".to_string(),
//!             children: vec![]
//!         }
//!     ]
//! };
//!
//! my_tree.drive_mut(&mut Renamer{from: "old", to: "new"});
//!
//! assert_eq!(my_tree.name, "new parent");
//! assert_eq!(my_tree.children[0].name, "new child");
//! ```
//!
//! ## Features
//! - `std-types-drive` - implement [Drive](Drive) for primitive types and String type from std.
//! It is [recommended](https://github.com/nikis05/derive-visitor/issues/3#issuecomment-1186690655) to
//! either skip these types in your `Drive` implementation, or to wrap them with newtypes, so this feature
//! is disabled by default. However it might be useful when driving through autogenerated structs.

/// See [`Drive`].
pub use derive_visitor_macros::Drive;

/// See [`DriveMut`].
pub use derive_visitor_macros::DriveMut;

/// See [`Visitor`].
pub use derive_visitor_macros::Visitor;

/// See [`VisitorMut`].
pub use derive_visitor_macros::VisitorMut;

use std::{any::Any, cell::Cell, marker::PhantomData};

#[cfg(feature = "std-types-drive")]
use std::ops::{Range, RangeBounds, RangeFrom, RangeInclusive, RangeTo, RangeToInclusive};
use std::sync::{Arc, Mutex, RwLock};

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

impl<V: Visitor> Visitor for &mut V {
    fn visit(&mut self, obj: &dyn Any, event: Event) {
        (**self).visit(obj, event)
    }
}

/// An interface for visiting data structures and mutating them during the visit.
///
/// It works exactly the same as [Visitor], but it takes a mutable reference to the visited element.
///
/// ```rust
/// use derive_visitor::VisitorMut;
///
/// struct Chain {
///     next: Option<Box<Chain>>
/// }
///
/// #[derive(VisitorMut)]
/// #[visitor(Chain(enter))]
/// struct ChainCutter;
///
/// impl ChainCutter {
///     fn enter_chain(&mut self, item: &mut Chain) {
///         item.next = None
///     }
/// }
/// ```
pub trait VisitorMut {
    fn visit(&mut self, item: &mut dyn Any, event: Event);
}

impl<V: VisitorMut> VisitorMut for &mut V {
    fn visit(&mut self, obj: &mut dyn Any, event: Event) {
        (**self).visit(obj, event)
    }
}

/// Create a visitor that only visits items of some specific type from a function or a closure.
///
/// ## Example
/// ```rust
/// use derive_visitor::{visitor_fn, Drive};
/// # #[derive(Drive)] struct File;
/// File.drive(&mut visitor_fn(|file: &File, event| {
///     // ...your logic here
/// }));
/// ```
pub fn visitor_fn<T, F: FnMut(&T, Event)>(fun: F) -> FnVisitor<T, F> {
    FnVisitor {
        marker: PhantomData,
        fun,
    }
}

/// Create a visitor that only visits items and mutates them with the given function
///
/// ## Example
/// ```rust
/// use derive_visitor::{visitor_fn_mut, DriveMut};
/// # #[derive(DriveMut)] struct File;
/// File.drive_mut(&mut visitor_fn_mut(|file: &mut File, event| {
///     // ...your logic here
/// }));
/// ```
pub fn visitor_fn_mut<T, F: FnMut(&mut T, Event)>(fun: F) -> FnVisitor<T, F> {
    FnVisitor {
        marker: PhantomData,
        fun,
    }
}

/// Similar to [visitor_fn](visitor_fn), but the closure will only be called on [Event::Enter](Event::Enter).
pub fn visitor_enter_fn<T, F: FnMut(&T)>(mut fun: F) -> FnVisitor<T, impl FnMut(&T, Event)> {
    visitor_fn(move |item, event| {
        if let Event::Enter = event {
            fun(item);
        }
    })
}

/// Similar to [`visitor_fn_mut`], but the closure will only be called on [Event::Enter](Event::Enter).
pub fn visitor_enter_fn_mut<T, F: FnMut(&mut T)>(
    mut fun: F,
) -> FnVisitor<T, impl FnMut(&mut T, Event)> {
    visitor_fn_mut(move |item, event| {
        if let Event::Enter = event {
            fun(item);
        }
    })
}

/// Type returned by [visitor_fn](visitor_fn).
pub struct FnVisitor<T, F> {
    marker: PhantomData<T>,
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

impl<T: Any, F: FnMut(&mut T, Event)> VisitorMut for FnVisitor<T, F> {
    fn visit(&mut self, item: &mut dyn Any, event: Event) {
        if let Some(item) = <dyn Any>::downcast_mut::<T>(item) {
            let fun = &mut self.fun;
            fun(item, event);
        }
    }
}

/// Defines whether an item is being entered or exited by a visitor.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Event {
    Enter,
    Exit,
}

/// A data structure that can drive a [visitor](Visitor) through itself.
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

/// Drive a [`VisitorMut`] over this datastructure.
///
/// This is equivalent to [`Drive`], but gives the possibility to mutate the datastructure as it is visited.
///
/// ## Example
///
/// ```rust
/// use derive_visitor::{DriveMut, Event, visitor_fn_mut};
///
/// #[derive(DriveMut, Default)]
/// struct Node{ children: Vec<Node> }
///
/// let mut node = Node{children: vec![Node::default(), Node::default()]};
///
/// node.drive_mut(&mut visitor_fn_mut(|n: &mut Node, event|
///     // Mutate the element on exit so that we are not driven to the newly created elements
///     if let Event::Exit = event {
///         n.children.resize_with(3, Default::default);
///     }
/// ));
///
/// // We have driven over all the initial nodes...
/// assert_eq!(node.children.len(), 3);
/// assert_eq!(node.children[0].children.len(), 3);
/// assert_eq!(node.children[1].children.len(), 3);
/// // ... but not over the newly created ones
/// assert_eq!(node.children[2].children.len(), 0);
/// ```
pub trait DriveMut: Any {
    fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V);
}

// Helper trait to the generic `IntoIterator` Drive impl
trait DerefAndDrive {
    fn deref_and_drive<V: Visitor>(self, visitor: &mut V);
}

// Drives a VisitorMut over a mutable reference
trait DerefAndDriveMut {
    fn deref_and_drive_mut<V: VisitorMut>(self, visitor: &mut V);
}

// Most collections iterate over item references, this is the trait impl that handles that case
impl<T: Drive> DerefAndDrive for &T {
    fn deref_and_drive<V: Visitor>(self, visitor: &mut V) {
        self.drive(visitor);
    }
}

impl<T: DriveMut> DerefAndDriveMut for &mut T {
    fn deref_and_drive_mut<V: VisitorMut>(self, visitor: &mut V) {
        self.drive_mut(visitor);
    }
}

// Map-like collections iterate over item references pairs
impl<TK: Drive, TV: Drive> DerefAndDrive for (&TK, &TV) {
    fn deref_and_drive<V: Visitor>(self, visitor: &mut V) {
        self.0.drive(visitor);
        self.1.drive(visitor);
    }
}

// Map-like collections have mutable iterators that allow mutating only the value, not the key
impl<TK, TV: DriveMut> DerefAndDriveMut for (TK, &mut TV) {
    fn deref_and_drive_mut<V: VisitorMut>(self, visitor: &mut V) {
        self.1.drive_mut(visitor);
    }
}

// Implement Drive and DriveMut for container types in standard library.
macro_rules! impl_drive_for_into_iterator {
    ( $type:ty ; $($generics:tt)+ ) => {
        impl< $($generics)+ > Drive for $type
        where
            $type: 'static,
            for<'a> &'a $type: IntoIterator,
            for<'a> <&'a $type as IntoIterator>::Item: DerefAndDrive,
        {
            fn drive<V: Visitor>(&self, visitor: &mut V) {
                for item in self {
                    item.deref_and_drive(visitor);
                }
            }
        }

        impl< $($generics)+ > DriveMut for $type
        where
            $type: 'static,
            for<'a> &'a mut $type: IntoIterator,
            for<'a> <&'a mut $type as IntoIterator>::Item: DerefAndDriveMut,
        {
            fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
                for item in self {
                    item.deref_and_drive_mut(visitor);
                }
            }
        }
    };
}

impl_drive_for_into_iterator! { [T] ; T }
impl_drive_for_into_iterator! { Vec<T> ; T }
impl_drive_for_into_iterator! { std::collections::BTreeSet<T> ; T }
impl_drive_for_into_iterator! { std::collections::BinaryHeap<T> ; T }
impl_drive_for_into_iterator! { std::collections::HashSet<T> ; T }
impl_drive_for_into_iterator! { std::collections::LinkedList<T> ; T }
impl_drive_for_into_iterator! { std::collections::VecDeque<T> ; T }
impl_drive_for_into_iterator! { Option<T> ; T }
impl_drive_for_into_iterator! { Result<T, U> ; T, U }
impl_drive_for_into_iterator! { std::collections::BTreeMap<T, U> ; T, U }
impl_drive_for_into_iterator! { std::collections::HashMap<T, U> ; T, U }
impl_drive_for_into_iterator! { [T; N] ; T, const N: usize }

impl<T> Drive for Box<T>
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        (**self).drive(visitor);
    }
}

impl<T> DriveMut for Box<T>
where
    T: DriveMut,
{
    fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
        (**self).drive_mut(visitor);
    }
}

impl<T> Drive for Arc<T>
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        (**self).drive(visitor);
    }
}

impl<T> Drive for Mutex<T>
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        let lock = self.lock().unwrap();
        lock.drive(visitor);
    }
}

impl<T> Drive for RwLock<T>
where
    T: Drive,
{
    fn drive<V: Visitor>(&self, visitor: &mut V) {
        let lock = self.read().unwrap();
        lock.drive(visitor);
    }
}

impl<T> DriveMut for Arc<Mutex<T>>
where
    T: DriveMut,
{
    fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
        let mut lock = self.lock().unwrap();
        lock.drive_mut(visitor);
    }
}

impl<T> DriveMut for Arc<RwLock<T>>
where
    T: DriveMut,
{
    fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
        let mut lock = self.write().unwrap();
        lock.drive_mut(visitor);
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

impl<T> DriveMut for Cell<T>
where
    T: DriveMut,
{
    fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
        self.get_mut().drive_mut(visitor);
    }
}

impl Drive for () {
    fn drive<V: Visitor>(&self, _visitor: &mut V) {}
}

impl DriveMut for () {
    fn drive_mut<V: VisitorMut>(&mut self, _visitor: &mut V) {}
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

            impl<$( $type ),+> DriveMut for ($($type,)+)
            where
                $(
                    $type: DriveMut
                ),+
            {
                fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
                    $(
                        self.$field.drive_mut(visitor);
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

#[cfg(feature = "std-types-drive")]
macro_rules! trivial_impl {
    ( $type:ty ) => {
        impl Drive for $type {
            fn drive<V: Visitor>(&self, visitor: &mut V) {
                visitor.visit(self, Event::Enter);
                visitor.visit(self, Event::Exit);
            }
        }
        impl DriveMut for $type {
            fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
                visitor.visit(self, Event::Enter);
                visitor.visit(self, Event::Exit);
            }
        }
    };
}

#[cfg(not(feature = "std-types-drive"))]
macro_rules! trivial_impl {
    ( $type:ident ) => {};
}

trivial_impl!(u8);
trivial_impl!(u16);
trivial_impl!(u32);
trivial_impl!(u64);
trivial_impl!(u128);
trivial_impl!(usize);

trivial_impl!(i8);
trivial_impl!(i16);
trivial_impl!(i32);
trivial_impl!(i64);
trivial_impl!(i128);
trivial_impl!(isize);

trivial_impl!(f32);
trivial_impl!(f64);

trivial_impl!(char);
trivial_impl!(bool);

trivial_impl!(String);

#[cfg(feature = "std-types-drive")]
mod drive_ranges {
    use super::*;
    use std::ops::*;

    impl<T: Drive> Drive for Range<T> {
        fn drive<V: Visitor>(&self, visitor: &mut V) {
            self.start.drive(visitor);
            self.end.drive(visitor);
        }
    }

    impl<T: DriveMut> DriveMut for Range<T> {
        fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
            self.start.drive_mut(visitor);
            self.end.drive_mut(visitor);
        }
    }

    impl<T: Drive> Drive for RangeTo<T> {
        fn drive<V: Visitor>(&self, visitor: &mut V) {
            self.end.drive(visitor);
        }
    }

    impl<T: DriveMut> DriveMut for RangeTo<T> {
        fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
            self.end.drive_mut(visitor);
        }
    }

    impl<T: Drive> Drive for RangeToInclusive<T> {
        fn drive<V: Visitor>(&self, visitor: &mut V) {
            self.end.drive(visitor);
        }
    }

    impl<T: DriveMut> DriveMut for RangeToInclusive<T> {
        fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
            self.end.drive_mut(visitor);
        }
    }

    impl<T: Drive> Drive for RangeFrom<T> {
        fn drive<V: Visitor>(&self, visitor: &mut V) {
            self.start.drive(visitor);
        }
    }

    impl<T: DriveMut> DriveMut for RangeFrom<T> {
        fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
            self.start.drive_mut(visitor);
        }
    }

    impl<T: Drive> Drive for RangeInclusive<T> {
        fn drive<V: Visitor>(&self, visitor: &mut V) {
            self.start().drive(visitor);
            self.end().drive(visitor);
        }
    }

    // Unfortunately, RangeInclusive does not give mutable access to its bounds, so we have to
    // add a T: Default constraint in order to have something to put into the old range while we are changing the bounds.
    // This should not cause issues in practice, because ranges of non-Default values are rare.
    impl<T: DriveMut> DriveMut for RangeInclusive<T>
    where
        T: Default,
    {
        fn drive_mut<V: VisitorMut>(&mut self, visitor: &mut V) {
            let placeholder = RangeInclusive::new(T::default(), T::default());
            let bounds = std::mem::replace(self, placeholder);
            let mut tuple = bounds.into_inner();
            tuple.drive_mut(visitor);
            *self = RangeInclusive::new(tuple.0, tuple.1);
        }
    }
}
