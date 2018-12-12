//! Adapts types to implement data_models::typed_value_tree

use super::data_models::typed_value_tree::{ListView, ListVisitor, MapVisitor, TypeView};

/// Implement this for Terminal / Primitive types to be treated as byte sequences
pub trait Terminal {
    fn get_id() -> u128;
    /// Must be platform independent
    fn bytes(&self) -> Vec<u8>;
}

/// Implement this for Struct / Aggregate types
pub trait Struct {
    fn get_id() -> u128;
    fn visit<V: MapVisitor<N = u128>>(&self, v: &mut V);
}

#[macro_export]
macro_rules! TypeViewForTerminal {
    ( $Type:ty ) => {
        impl TypeView for $Type {
            type N = u128;

            fn visit<V: TypeVisitor<N = Self::N>>(&self, v: &mut V) {
                v.visit_value(&<Self as Terminal>::get_id(), &self.bytes());
            }
        }

        impl Named for $Type {
            fn get_id() -> u128 {
                <Self as Terminal>::get_id()
            }
        }
    };
}

#[macro_export]
macro_rules! TypeViewForStruct {
    ( $Type:ty ) => {
        impl TypeView for $Type {
            type N = u128;

            fn visit<V: TypeVisitor<N = Self::N>>(&self, v: &mut V) {
                v.visit_map(&<Self as Struct>::get_id(), self);
            }
        }

        impl MapView for $Type {
            type N = u128;

            fn visit<V: MapVisitor<N = u128>>(&self, v: &mut V) {
                <Self as Struct>::visit(self, v);
            }
        }

        impl Named for $Type {
            fn get_id() -> u128 {
                <Self as Struct>::get_id()
            }
        }
    };
}

pub trait Named {
    fn get_id() -> u128;
}

pub fn visit_single_field<T, V>(v: &mut V, name: &u128, t: &T)
where
    T: TypeView<N = u128>,
    V: MapVisitor<N = u128>,
{
    v.visit(name, &ContentListerVisiter(t));

    struct ContentListerVisiter<T>(T);
    impl<T> ListView for ContentListerVisiter<&T>
    where
        T: TypeView<N = u128>,
    {
        type N = u128;
        fn visit<V: ListVisitor<N = Self::N>>(&self, v: &mut V) {
            v.visit(self.0);
        }
    }
}

// TODO: make this accept any IntoIterator not just Vec
pub fn visit_list_field<T, V>(v: &mut V, name: &u128, t: &Vec<T>)
where
    T: TypeView<N = u128>,
    V: MapVisitor<N = u128>,
{
    v.visit(name, &ContentListerVisiter(t));

    struct ContentListerVisiter<'a, T>(&'a Vec<T>);
    impl<'a, T> ListView for ContentListerVisiter<'a, T>
    where
        T: TypeView<N = u128>,
    {
        type N = u128;
        fn visit<V: ListVisitor<N = Self::N>>(&self, v: &mut V) {
            for child in self.0 {
                v.visit(child);
            }
        }
    }
}
