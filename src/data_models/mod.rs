//! Data Models used in this library.
//! Since there is no single Trait that an object will implement to expose itself as one of these data models,
//! the names for the models are used by nested modules.
//! Each nested module contains a reference concrete implementations with minimal complexity
//! as well as traits for generic implementations/adapters based on the visitor pattern.
//!
//! These modules do not depend on each other, and adapters/converts are separate.

#[macro_export]
macro_rules! Visitable {
    ( $Name:ident, $Visitor:ident, $($Items:ident);* ) => {
        pub trait $Name {
            $(type $Items;)*
            fn visit<V: $Visitor<$($Items = Self::$Items),*>>(&self, v: &mut V);
            fn apply<V: $Visitor<$($Items = Self::$Items),*>>(&self, mut v: V) -> V {
                self.visit(&mut v);
                return v;
            }
        }
    };
}

pub mod leaf_tree;

#[macro_use]
pub mod typed_value_tree;
