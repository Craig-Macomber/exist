//! Data Models used in this library.
//! Since there is no single Trait that an object will implement to expose itself as one of these data models,
//! the names for the models are used by nested modules.
//! Each nested module contains a reference concrete implementations with minimal complexity
//! as well as traits for generic implementations/adapters based on the visitor pattern.
//!
//! These modules do not depend on each other, and adapters/converts are separate.

#[macro_export]
macro_rules! Visitable {
    ( $Name:ident,$Visitor:path, $($SuperTrait:ident)*; $($Items:ident);* ) => {
        pub trait $Name: $($SuperTrait)* {
            $(type $Items;)*
            fn visit<V: $Visitor>(&self, v: &mut V);
            fn apply<V: $Visitor>(&self, mut v: V) -> V {
                self.visit(&mut v);
                return v;
            }
        }
    };
}

/// A n-ary tree with data (other than structure) only in the leaves.
/// This is the simplest concrete implementation, but the same name may be used for
/// the concept and/or interfaces sharing this data-model.
///
/// Generally, LeafTrees are a simplifying encoding or view of a more semantic structure.
pub mod leaf_tree {
    pub enum Concrete<V> {
        List(Vec<Concrete<V>>),
        Value(V),
    }

    Visitable!(View, Visitor<Value = Self::Value>,; Value);

    pub trait Visitor {
        type Value;

        /// Called for with each child in order when visiting a list node
        fn visit_list<T: View<Value = Self::Value>>(&mut self, t: &T);

        /// Called once with the value when visiting a value node
        fn visit_value(&mut self, t: Self::Value);
    }

    impl<Value> View for Concrete<Value>
    where
        Value: Clone,
    {
        type Value = Value;
        fn visit<V: Visitor<Value = Self::Value>>(&self, v: &mut V) {
            match self {
                Concrete::List(list) => {
                    for c in list {
                        v.visit_list(c);
                    }
                }
                Concrete::Value(value) => {
                    v.visit_value(value.clone());
                }
            };
        }
    }

    // Copy into the standard Concrete implementation
    pub fn view_to_concrete<T, V>(t: T) -> Concrete<V>
    where
        T: View<Value = V>,
        V: Clone,
    {
        impl<V> Visitor for Vec<Concrete<V>> {
            type Value = V;
            fn visit_list<T: View<Value = V>>(&mut self, t: &T) {
                self.push(Concrete::List(t.apply(vec![])));
            }
            fn visit_value(&mut self, v: V) {
                self.push(Concrete::Value(v));
            }
        }

        return t.apply(vec![]).into_iter().nth(0).unwrap();
    }
}

/// Some data models between TypedValueTree and LeafTree that could be useful abstractions,
/// but are currently unused.
mod misc_data_models {
    use std::collections::HashMap;

    /// A n-ary tree where children are grouped into ordered sequences under keys.
    /// All data (other than structure) is in the names.
    pub struct NameTree<N> {
        children: HashMap<N, Vec<NameTree<N>>>,
    }

    /// NameTree with type_names on the nodes.
    pub struct TypedTree<TN, N> {
        type_name: TN,
        children: HashMap<N, Vec<TypedTree<TN, N>>>,
    }
}

pub mod typed_value_tree;
