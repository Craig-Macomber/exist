//! TypedTree, but can store value direct in the nodes instead of just children.
//!
//! Associated types were getting too verbose. Shortened to:
//! TN = TypeName
//! CN = ChildName
//! Val = Val
//!
//! When listing all three, keep them in that order.
//!
//! V = Type parameter that is a Visitor
//! T = Type parameter that is a Tree (might just be a value in a Tree though)

use super::DefaultTag;
use std::collections::HashMap;
use std::hash::Hash;

pub struct Concrete<TN, CN, Val>
where
    CN: Eq + Hash,
    Val: Clone,
{
    type_name: TN,
    content: StructOrValue<TN, CN, Val>,
}

/// Content helper for Concrete.
pub enum StructOrValue<TN, CN, Val>
where
    CN: Eq + Hash,
    Val: Clone,
{
    Struct(HashMap<CN, Vec<Concrete<TN, CN, Val>>>),
    Val(Vec<Val>),
}

/// Helper super trait for provide the associated types used in this module
pub trait Base<Tag> {
    type TN;
    type CN;
    type Val;
}

#[macro_export]
macro_rules! Associated {
    ( $Name:path ) => {
        $Name<TN = Self::TN, CN = Self::CN, Val = Self::Val>
    };
}

/// Defines a trait requiring visit and providing apply.
/// Provided Visitor should be a trait extending Base
#[macro_export]
macro_rules! Visitable2 {
    ( $Name:ident,$Visitor:ident) => {
        Visitable!($Name, $Visitor, TN;CN;Val);
    };
}

Visitable2!(TypeView, TypeVisitor);
Visitable2!(MapView, MapVisitor);
Visitable2!(ListView, ListVisitor);

pub trait TypeVisitor<Tag>: Base<Tag> {
    /// Called if the View was a Struct (map)
    fn visit_map<T: MapView<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
        &mut self,
        type_name: &Self::TN,
        t: &T,
    );

    /// Called if the View was a Val / Leaf
    // TODO: use ExactSizeIterator?
    fn visit_value(&mut self, type_name: &Self::TN, t: &Vec<Self::Val>);
}

pub trait MapVisitor<Tag>: Base<Tag> {
    /// Called for with value in the map
    fn visit<T: ListView<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
        &mut self,
        name: &Self::CN,
        children: &T,
    );
}

pub trait ListVisitor<Tag>: Base<Tag> {
    /// Called for with value in the children list in order
    fn visit<T: TypeView<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&mut self, child: &T);
}

/// Helper for reducing boilerplate when implementing Visitable traits (TypeView, MapView, ListView)
/// T must use type parameters <TN, CN, Val>
#[macro_export]
macro_rules! ImplVisitable {
    ( $Visitable:ident, $Tag:ty, $T:ty, $Visitor:ident, $V:expr ) => {
        impl<TN, CN, Val> $Visitable<$Tag> for $T
        where
            Val: Clone,
            CN: Eq + Hash,
        {
            type TN = TN;
            type CN = CN;
            type Val = Val;

            fn visit<V: $Visitor<$Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
                &self,
                v: &mut V,
            ) {
                $V(self, v);
            }
        }
    };
}

ImplVisitable!(
    TypeView,
    DefaultTag,
    Concrete<TN, CN, Val>,
    TypeVisitor,
    |this: &Self, v: &mut V| -> () {
        match &this.content {
            StructOrValue::Struct(map) => {
                v.visit_map(&this.type_name, map);
            }
            StructOrValue::Val(value) => {
                v.visit_value(&this.type_name, value);
            }
        };
    }
);

ImplVisitable!(
    MapView,
    DefaultTag,
    HashMap<CN, Vec<Concrete<TN, CN, Val>>>,
    MapVisitor,
    |this: &Self, v: &mut V| -> () {
        for (k, children) in this.iter() {
            v.visit(&k, children);
        }
    }
);

ImplVisitable!(
    ListView,
    DefaultTag,
    Vec<Concrete<TN, CN, Val>>,
    ListVisitor,
    |this: &Self, v: &mut V| -> () {
        for child in this.iter() {
            v.visit(child);
        }
    }
);

// impl<TN, CN, Val> TypeView for Concrete<TN, CN, Val>
// where
//     Val: Clone,
//     CN: Eq + Hash,
// {
//     fn visit<V: TypeVisitor<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&self, v: &mut V) {
//         match &self.content {
//             StructOrValue::Struct(map) => {
//                 v.visit_map(&self.type_name, map);
//             }
//             StructOrValue::Val(value) => {
//                 v.visit_value(&self.type_name, value);
//             }
//         };
//     }
// }

// impl<Val, TN, CN> MapView for HashMap<CN, Vec<Concrete<TN, CN, Val>>>
// where
//     Val: Clone,
//     CN: Eq + Hash,
// {
//     fn visit<V: MapVisitor<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&self, v: &mut V) {
//         for (k, children) in self.iter() {
//             v.visit(&k, children);
//         }
//     }
// }

// impl<Val, TN, CN> ListView for Vec<Concrete<TN, CN, Val>>
// where
//     Val: Clone,
//     CN: Eq + Hash,
// {
//     fn visit<V: ListVisitor<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&self, v: &mut V) {
//         for child in self.iter() {
//             v.visit(child);
//         }
//     }
// }

/// Copy into the standard Concrete implementation
pub fn view_to_concrete<T, Tag, TN, CN, Val>(t: T) -> Concrete<TN, CN, Val>
where
    T: TypeView<Tag, Val = Val, TN = TN, CN = CN>,
    Val: Clone,
    TN: Clone,
    CN: Clone + Eq + Hash,
{
    struct Copier<T> {
        t: T,
    }

    fn copier<T>(t: T) -> Copier<T> {
        Copier { t }
    }

    type Co<TN, CN, Val> = Copier<Option<Concrete<TN, CN, Val>>>;
    type Cm<TN, CN, Val> = Copier<HashMap<CN, Vec<Concrete<TN, CN, Val>>>>;
    type Cv<TN, CN, Val> = Copier<Vec<Concrete<TN, CN, Val>>>;

    impl<Tag, TN, CN, Val> Base<Tag> for Co<TN, CN, Val>
    where
        Val: Clone,
        CN: Eq + Hash,
    {
        type TN = TN;
        type CN = CN;
        type Val = Val;
    }

    impl<Tag, TN, CN, Val> Base<Tag> for Cm<TN, CN, Val>
    where
        Val: Clone,
        CN: Eq + Hash,
    {
        type TN = TN;
        type CN = CN;
        type Val = Val;
    }

    impl<Tag, TN, CN, Val> Base<Tag> for Cv<TN, CN, Val>
    where
        Val: Clone,
        CN: Eq + Hash,
    {
        type TN = TN;
        type CN = CN;
        type Val = Val;
    }

    impl<Tag, TN, CN, Val> TypeVisitor<Tag> for Co<TN, CN, Val>
    where
        Val: Clone,
        TN: Clone,
        CN: Clone + Eq + Hash,
    {
        fn visit_map<T: MapView<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &mut self,
            type_name: &Self::TN,
            t: &T,
        ) {
            self.t = Some(Concrete {
                type_name: type_name.clone(),
                content: StructOrValue::Struct(t.apply(copier(HashMap::new())).t),
            });
        }
        fn visit_value(&mut self, type_name: &Self::TN, t: &Vec<Self::Val>) {
            self.t = Some(Concrete {
                type_name: type_name.clone(),
                content: StructOrValue::Val(t.clone()),
            })
        }
    }

    impl<Tag, TN, CN, Val> MapVisitor<Tag> for Cm<TN, CN, Val>
    where
        Val: Clone,
        TN: Clone,
        CN: Clone + Eq + Hash,
    {
        fn visit<T: ListView<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &mut self,
            name: &Self::CN,
            children: &T,
        ) {
            self.t
                .insert(name.clone(), children.apply(copier(vec![])).t);
        }
    }

    impl<Tag, TN, CN, Val> ListVisitor<Tag> for Cv<TN, CN, Val>
    where
        Val: Clone,
        TN: Clone,
        CN: Clone + Eq + Hash,
    {
        fn visit<T: TypeView<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &mut self,
            child: &T,
        ) {
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
                type_name: 3i64,
                content: StructOrValue::Val(vec![1u8, 2u8, 3u8]),
            }],
        );
        let c = Concrete {
            type_name: 2i64,
            content: StructOrValue::Struct(map),
        };

        let c2 = view_to_concrete(c);
        //assert_eq!(c, c2);
    }
}
