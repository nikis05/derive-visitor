#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub use derive_visitor_macros::{Visitor, Walk};
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
    fn drive(&mut self, item: &dyn Any, op: Op);
}

pub trait Walk: Any {
    fn walk<V: Visitor>(&self, visitor: &mut V);
}

impl<K, Val> Walk for BTreeMap<K, Val>
where
    K: Walk,
    Val: Walk,
{
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|(key, value)| {
            key.walk(visitor);
            value.walk(visitor);
        });
    }
}

impl<T> Walk for BTreeSet<T>
where
    T: Walk,
{
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.walk(visitor));
    }
}

impl<T> Walk for BinaryHeap<T>
where
    T: Walk,
{
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.walk(visitor));
    }
}

impl<T> Walk for Box<T>
where
    T: Walk,
{
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        (**self).walk(visitor);
    }
}

impl<T> Walk for Cell<T>
where
    T: Walk + Copy,
{
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        self.get().walk(visitor);
    }
}

impl<K, Val, S> Walk for HashMap<K, Val, S>
where
    K: Walk,
    Val: Walk,
    S: 'static,
{
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|(key, value)| {
            key.walk(visitor);
            value.walk(visitor);
        });
    }
}

impl<T, S> Walk for HashSet<T, S>
where
    T: Walk,
    S: 'static,
{
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.walk(visitor));
    }
}

impl<T> Walk for LinkedList<T>
where
    T: Walk,
{
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.walk(visitor));
    }
}

impl<T> Walk for Option<T>
where
    T: Walk,
{
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        if let Some(value) = self {
            value.walk(visitor);
        }
    }
}

impl<T> Walk for Vec<T>
where
    T: Walk,
{
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.walk(visitor));
    }
}

impl<T> Walk for VecDeque<T>
where
    T: Walk,
{
    fn walk<V: Visitor>(&self, visitor: &mut V) {
        self.iter().for_each(|item| item.walk(visitor));
    }
}

impl Walk for () {
    fn walk<V: Visitor>(&self, _visitor: &mut V) {}
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
                fn walk<V: Visitor>(&self, visitor: &mut V) {
                    $(
                        self.$field.walk(visitor);
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

impl<T> Walk for [T; 0]
where
    T: Walk,
{
    fn walk<V: Visitor>(&self, _visitor: &mut V) {}
}

macro_rules! array_impls {
    ( $( $len:expr => $( $field:expr ),+ )+ ) => {
        $(
            impl<T> Walk for [T; $len]
            where
                T: Walk
            {
                fn walk<V: Visitor>(&self, visitor: &mut V) {
                    $(
                        self[$field].walk(visitor);
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
