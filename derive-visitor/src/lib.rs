#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub use derive_visitor_macros::{Drive, Visitor};
use std::{
    any::Any,
    cell::Cell,
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque},
};

pub enum Op {
    Enter,
    Exit,
}

pub trait Visitor {
    fn visit(&mut self, item: &dyn Any, op: Op);
}

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
