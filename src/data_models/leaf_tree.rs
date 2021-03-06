//! A n-ary tree with data (other than structure) only in the leaves.
//! This is the simplest concrete implementation, but the same name may be used for
//! the concept and/or interfaces sharing this data-model.
//!
//! Generally, LeafTrees are a simplifying encoding or view of a more semantic structure.

Visitable!(View, Visitor, Value);

pub trait Visitor {
    type Value;

    /// Called for with each child in order when visiting a list node
    fn visit_list<T: View<Value = Self::Value>>(&mut self, t: &T);

    /// Called once with the value when visiting a value node
    fn visit_value(&mut self, t: Self::Value);
}

pub mod concrete {
    use super::*;

    // TODO: remove need for Clone?
    #[derive(Debug, PartialEq, Eq, Clone, Hash)]
    pub enum Concrete<V> {
        List(Vec<Concrete<V>>),
        Value(V),
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
    pub fn view_to_concrete<T, V>(t: &T) -> Concrete<V>
    where
        T: View<Value = V>,
        V: Clone,
    {
        struct Out<V>(Concrete<V>);
        impl<V> Visitor for Out<V>
        where
            V: Clone,
        {
            type Value = V;
            fn visit_list<T: View<Value = V>>(&mut self, t: &T) {
                match &mut self.0 {
                    Concrete::List(vec) => vec.push(view_to_concrete(t)),
                    _ => panic!(),
                };
            }
            fn visit_value(&mut self, v: V) {
                match &self.0 {
                    Concrete::List(vec) => {
                        assert_eq!(vec.len(), 0);
                    }
                    _ => panic!(),
                };

                self.0 = Concrete::Value(v);
            }
        }

        return t.apply(Out(Concrete::List(vec![]))).0;
    }
}
