#[cfg(feature = "std-types-walk")]
use std::ops::{Range, RangeFrom, RangeInclusive, RangeTo, RangeToInclusive};
use std::{cell::Cell, sync::{Arc, Mutex, RwLock}};
use crate::{Visitor, VisitorMut, Walk, WalkMut};

// Helper trait to the generic `IntoIterator` Walk impl
trait DerefAndWalk<'visitor> {
    fn deref_and_walk<V: Visitor<'visitor> + ?Sized>(self, visitor: &mut V);
}

// Drives a VisitorMut over a mutable reference
trait DerefAndWalkMut {
    fn deref_and_walk_mut<V: VisitorMut + ?Sized>(self, visitor: &mut V);
}

// Most collections iterate over item references, this is the trait impl that handles that case
impl<'visitor, T: Walk> DerefAndWalk<'visitor> for &'visitor T {
    fn deref_and_walk<V: Visitor<'visitor> + ?Sized>(self, visitor: &mut V) {
        self.walk(visitor);
    }
}

impl<T: WalkMut> DerefAndWalkMut for &mut T {
    fn deref_and_walk_mut<V: VisitorMut + ?Sized>(self, visitor: &mut V) {
        self.walk_mut(visitor);
    }
}

// Map-like collections iterate over item references pairs
impl<'visitor, TK: Walk, TV: Walk> DerefAndWalk<'visitor> for (&'visitor TK, &'visitor TV) {
    fn deref_and_walk<V: Visitor<'visitor> + ?Sized>(self, visitor: &mut V) {
        self.0.walk(visitor);
        self.1.walk(visitor);
    }
}

// Map-like collections have mutable iterators that allow mutating only the value, not the key
impl<TK, TV: WalkMut> DerefAndWalkMut for (TK, &mut TV) {
    fn deref_and_walk_mut<V: VisitorMut + ?Sized>(self, visitor: &mut V) {
        self.1.walk_mut(visitor);
    }
}

// Implement Walk and WalkMut for container types in standard library.
macro_rules! impl_walk_for_into_iterator {
    ( $type:ty ; $($generics:tt)+ ) => {
        impl< $($generics)+ > Walk for $type
        where
            $type: 'static,
            for<'a> &'a $type: IntoIterator,
            for<'a> <&'a $type as IntoIterator>::Item: DerefAndWalk<'a>,
        {
            fn walk<'visitor, V: Visitor<'visitor> + ?Sized>(&'visitor self, visitor: &mut V)  {
                #[allow(for_loops_over_fallibles)]
                for item in self {
                    item.deref_and_walk(visitor);
                }
            }
        }

        impl< $($generics)+ > WalkMut for $type
        where
            $type: 'static,
            for<'a> &'a mut $type: IntoIterator,
            for<'a> <&'a mut $type as IntoIterator>::Item: DerefAndWalkMut,
        {
            fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
                #[allow(for_loops_over_fallibles)]
                for item in self {
                    item.deref_and_walk_mut(visitor);
                }
            }
        }
    };
}

impl_walk_for_into_iterator! { [T] ; T }
impl_walk_for_into_iterator! { Vec<T> ; T }
impl_walk_for_into_iterator! { std::collections::BTreeSet<T> ; T }
impl_walk_for_into_iterator! { std::collections::BinaryHeap<T> ; T }
impl_walk_for_into_iterator! { std::collections::HashSet<T> ; T }
impl_walk_for_into_iterator! { std::collections::LinkedList<T> ; T }
impl_walk_for_into_iterator! { std::collections::VecDeque<T> ; T }
impl_walk_for_into_iterator! { Option<T> ; T }
impl_walk_for_into_iterator! { Result<T, U> ; T, U }
impl_walk_for_into_iterator! { std::collections::BTreeMap<T, U> ; T, U }
impl_walk_for_into_iterator! { std::collections::HashMap<T, U> ; T, U }
impl_walk_for_into_iterator! { [T; N] ; T, const N: usize }

impl<T> Walk for Box<T>
where
    T: Walk,
{
    fn walk<'visitor, V: Visitor<'visitor> + ?Sized>(&'visitor self, visitor: &mut V) {
        (**self).walk(visitor);
    }
}

impl<T> WalkMut for Box<T>
where
    T: WalkMut,
{
    fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
        (**self).walk_mut(visitor);
    }
}

impl<T> Walk for Arc<T>
where
    T: Walk,
{
    fn walk<'visitor, V: Visitor<'visitor> + ?Sized>(&'visitor self, visitor: &mut V) {
        (**self).walk(visitor);
    }
}

impl<T> WalkMut for Arc<Mutex<T>>
where
    T: WalkMut,
{
    fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
        let mut lock = self.lock().unwrap();
        lock.walk_mut(visitor);
    }
}

impl<T> WalkMut for Arc<RwLock<T>>
where
    T: WalkMut,
{
    fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
        let mut lock = self.write().unwrap();
        lock.walk_mut(visitor);
    }
}

impl<T> WalkMut for Cell<T>
where
    T: WalkMut,
{
    fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
        self.get_mut().walk_mut(visitor);
    }
}

impl Walk for () {
    fn walk<'visitor, V: Visitor<'visitor> + ?Sized>(&self, _visitor: &mut V) {}
}

impl WalkMut for () {
    fn walk_mut<V: VisitorMut + ?Sized>(&mut self, _visitor: &mut V) {}
}

macro_rules! tuple_impls {
    ( $( $( $type:ident ),+ => $( $field:tt ),+ )+ ) => {
        $(
            impl<$( $type ),+> Walk for ($($type,)+)
            where
                $(
                    $type: Walk
                ),+
            {
                fn walk<'visitor, V: Visitor<'visitor> + ?Sized>(&'visitor self, visitor: &mut V) {
                    $(
                        self.$field.walk(visitor);
                    )+
                }
            }

            impl<$( $type ),+> WalkMut for ($($type,)+)
            where
                $(
                    $type: WalkMut
                ),+
            {
                fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
                    $(
                        self.$field.walk_mut(visitor);
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

#[cfg(feature = "std-types-walk")]
macro_rules! trivial_impl {
    ( $type:ty ) => {
        impl Walk for $type {
            fn walk<'visitor, V: Visitor<'visitor> + ?Sized>(&'visitor self, visitor: &mut V) {
                visitor.enter(self);
                visitor.exit(self);
            }
        }
        impl WalkMut for $type {
            fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
                visitor.enter(self);
                visitor.exit(self);
            }
        }
    };
}

#[cfg(not(feature = "std-types-walk"))]
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

#[cfg(feature = "std-types-walk")]
mod drive_ranges {
    use super::{Walk, Range, Visitor, WalkMut, VisitorMut, RangeTo, RangeToInclusive, RangeFrom, RangeInclusive};

    impl<T: Walk> Walk for Range<T> {
        fn walk<'visitor, V: Visitor<'visitor> + ?Sized>(&'visitor self, visitor: &mut V) {
            self.start.walk(visitor);
            self.end.walk(visitor);
        }
    }

    impl<T: WalkMut> WalkMut for Range<T> {
        fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
            self.start.walk_mut(visitor);
            self.end.walk_mut(visitor);
        }
    }

    impl<T: Walk> Walk for RangeTo<T> {
        fn walk<'visitor, V: Visitor<'visitor> + ?Sized>(&'visitor self, visitor: &mut V) {
            self.end.walk(visitor);
        }
    }

    impl<T: WalkMut> WalkMut for RangeTo<T> {
        fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
            self.end.walk_mut(visitor);
        }
    }

    impl<T: Walk> Walk for RangeToInclusive<T> {
        fn walk<'visitor, V: Visitor<'visitor> + ?Sized >(&'visitor self, visitor: &mut V) {
            self.end.walk(visitor);
        }
    }

    impl<T: WalkMut> WalkMut for RangeToInclusive<T> {
        fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
            self.end.walk_mut(visitor);
        }
    }

    impl<T: Walk> Walk for RangeFrom<T> {
        fn walk<'visitor, V: Visitor<'visitor> + ?Sized>(&'visitor self, visitor: &mut V) {
            self.start.walk(visitor);
        }
    }

    impl<T: WalkMut> WalkMut for RangeFrom<T> {
        fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
            self.start.walk_mut(visitor);
        }
    }

    impl<T: Walk> Walk for RangeInclusive<T> {
        fn walk<'visitor, V: Visitor<'visitor> + ?Sized>(&'visitor self, visitor: &mut V) {
            self.start().walk(visitor);
            self.end().walk(visitor);
        }
    }

    // Unfortunately, RangeInclusive does not give mutable access to its bounds, so we have to
    // add a T: Default constraint in order to have something to put into the old range while we are changing the bounds.
    // This should not cause issues in practice, because ranges of non-Default values are rare.
    impl<T: WalkMut> WalkMut for RangeInclusive<T>
    where
        T: Default,
    {
        fn walk_mut<V: VisitorMut + ?Sized>(&mut self, visitor: &mut V) {
            let placeholder = RangeInclusive::new(T::default(), T::default());
            let bounds = std::mem::replace(self, placeholder);
            let mut tuple = bounds.into_inner();
            tuple.walk_mut(visitor);
            *self = RangeInclusive::new(tuple.0, tuple.1);
        }
    }
}
