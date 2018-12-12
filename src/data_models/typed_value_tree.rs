//! TypedTree, but can store value direct in the nodes instead of just children.
//!
//! Types were getting too verbose. Shortened to:
//! N = Name
//! V = Type parameter that is a Visitor
//! T = Type parameter that is a Tree / Visitable

Visitable!(TypeView, TypeVisitor, N);
Visitable!(MapView, MapVisitor, N);
Visitable!(ListView, ListVisitor, N);

pub trait TypeVisitor {
    type N;
    /// Called if the View was a Struct (map)
    fn visit_map<T: MapView<N = Self::N>>(&mut self, type_name: &Self::N, t: &T);

    /// Called if the View was a Value / Leaf
    // TODO: use ExactSizeIterator?
    fn visit_value(&mut self, type_name: &Self::N, t: &[u8]);
}

pub trait MapVisitor {
    type N;
    /// Called for with value in the map
    fn visit<T: ListView<N = Self::N>>(&mut self, name: &Self::N, children: &T);
}

pub trait ListVisitor {
    type N;
    /// Called for with value in the children list in order
    fn visit<T: TypeView<N = Self::N>>(&mut self, child: &T);
}

pub mod concrete {
    use super::*;
    use std::collections::HashMap;
    use std::hash::Hash;

    #[derive(Debug, PartialEq)]
    pub struct Concrete<N>
    where
        N: Eq + Hash,
    {
        type_name: N,
        content: StructOrValue<N>,
    }

    /// Content helper for Concrete.
    #[derive(Debug, PartialEq)]
    pub enum StructOrValue<N>
    where
        N: Eq + Hash,
    {
        Struct(HashMap<N, Vec<Concrete<N>>>),
        Value(Vec<u8>),
    }

    impl<N> TypeView for Concrete<N>
    where
        N: Eq + Hash,
    {
        type N = N;
        fn visit<V: TypeVisitor<N = Self::N>>(&self, v: &mut V) {
            match &self.content {
                StructOrValue::Struct(map) => {
                    v.visit_map(&self.type_name, map);
                }
                StructOrValue::Value(value) => {
                    v.visit_value(&self.type_name, value);
                }
            };
        }
    }

    impl<N> MapView for HashMap<N, Vec<Concrete<N>>>
    where
        N: Eq + Hash,
    {
        type N = N;
        fn visit<V: MapVisitor<N = Self::N>>(&self, v: &mut V) {
            for (k, children) in self.iter() {
                v.visit(&k, children);
            }
        }
    }

    impl<N> ListView for Vec<Concrete<N>>
    where
        N: Eq + Hash,
    {
        type N = N;
        fn visit<V: ListVisitor<N = Self::N>>(&self, v: &mut V) {
            for child in self.iter() {
                v.visit(child);
            }
        }
    }

    /// Copy into the standard Concrete implementation
    pub fn view_to_concrete<T, N>(t: &T) -> Concrete<N>
    where
        T: TypeView<N = N>,
        N: Clone + Eq + Hash,
    {
        struct Copier<T> {
            t: T,
        }

        fn copier<T>(t: T) -> Copier<T> {
            Copier { t }
        }

        impl<N> TypeVisitor for Copier<Option<Concrete<N>>>
        where
            N: Clone + Eq + Hash,
        {
            type N = N;
            fn visit_map<T: MapView<N = Self::N>>(&mut self, type_name: &Self::N, t: &T) {
                self.t = Some(Concrete {
                    type_name: type_name.clone(),
                    content: StructOrValue::Struct(t.apply(copier(HashMap::new())).t),
                });
            }
            fn visit_value(&mut self, type_name: &Self::N, t: &[u8]) {
                self.t = Some(Concrete {
                    type_name: type_name.clone(),
                    content: StructOrValue::Value(t.to_vec()),
                })
            }
        }

        impl<N> MapVisitor for Copier<HashMap<N, Vec<Concrete<N>>>>
        where
            N: Clone + Eq + Hash,
        {
            type N = N;
            fn visit<T: ListView<N = Self::N>>(&mut self, name: &Self::N, children: &T) {
                self.t
                    .insert(name.clone(), children.apply(copier(vec![])).t);
            }
        }

        impl<N> ListVisitor for Copier<Vec<Concrete<N>>>
        where
            N: Clone + Eq + Hash,
        {
            type N = N;
            fn visit<T: TypeView<N = Self::N>>(&mut self, child: &T) {
                self.t.push(child.apply(copier(None)).t.unwrap());
            }
        }

        return t.apply(copier(None)).t.unwrap();
    }

    #[cfg(test)]
    mod tests {
        use super::{view_to_concrete, Concrete, StructOrValue};
        use std::collections::HashMap;

        #[test]
        fn test_view_to_concrete() {
            let mut map = HashMap::new();
            map.insert(
                1i128,
                vec![Concrete {
                    type_name: 1i128,
                    content: StructOrValue::Value(vec![1u8, 2u8, 3u8]),
                }],
            );
            let c = Concrete {
                type_name: 1i128,
                content: StructOrValue::Struct(map),
            };

            let c2 = view_to_concrete(&c);
            assert_eq!(c, c2);
        }
    }
}
