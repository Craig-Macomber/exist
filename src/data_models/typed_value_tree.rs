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
pub trait Base {
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
    ( $Name:ident,$Visitor:ident ) => {
        Visitable!($Name, $Visitor<TN = Self::TN, CN = Self::CN, Val = Self::Val>, Base;);
    };
}

Visitable2!(View, Visitor);
Visitable2!(MapView, MapVisitor);
Visitable2!(ListView, ListVisitor);

pub trait Visitor: Base {
    /// Called if the View was a Struct (map)
    fn visit_map<T: MapView<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
        &mut self,
        type_name: &Self::TN,
        t: &T,
    );

    /// Called if the View was a Val / Leaf
    // TODO: use ExactSizeIterator?
    fn visit_value(&mut self, type_name: &Self::TN, t: &Vec<Self::Val>);
}

pub trait MapVisitor: Base {
    /// Called for with value in the map
    fn visit<T: ListView<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
        &mut self,
        name: &Self::CN,
        children: &T,
    );
}

pub trait ListVisitor: Base {
    /// Called for with value in the children list in order
    fn visit<T: View<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&mut self, child: &T);
}

/// implements Base for T.
/// T must use type parameters <TN, CN, Val>
#[macro_export]
macro_rules! ImplBase {
    ( $T:ty ) => {
        impl<TN, CN, Val> Base for $T
        where
            Val: Clone,
            CN: Eq + Hash,
        {
            type TN = TN;
            type CN = CN;
            type Val = Val;
        }
    };
}

/// Helper for reducing boilerplate when implementing Visitable traits (View, MapView, ListView)
/// T must use type parameters <TN, CN, Val>
#[macro_export]
macro_rules! ImplVisitable {
    ( $Visitable:ty, $T:ty, $Visitor:ident, $V:expr ) => {
        impl<Val, TN, CN> $Visitable for $T
        where
            Val: Clone,
            CN: Eq + Hash,
        {
            fn visit<V: $Visitor<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&self, v: &mut V) {
                $V(self, v);
            }
        }
    };
}

#[macro_export]
macro_rules! ImplVisitableAndBase {
    ( $Visitable:ty, $T:ty, $Visitor:ident, $V:expr ) => {
        ImplBase!($T);
        ImplVisitable!($Visitable, $T, $Visitor, $V);
    };
}

ImplVisitableAndBase!(
    View,
    Concrete<TN, CN, Val>,
    Visitor,
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

ImplVisitableAndBase!(
    MapView,
    HashMap<CN, Vec<Concrete<TN, CN, Val>>>,
    MapVisitor,
    |this: &Self, v: &mut V| -> () {
        for (k, children) in this.iter() {
            v.visit(&k, children);
        }
    }
);

ImplVisitableAndBase!(
    ListView,
    Vec<Concrete<TN, CN, Val>>,
    ListVisitor,
    |this: &Self, v: &mut V| -> () {
        for child in this.iter() {
            v.visit(child);
        }
    }
);

/// Copy into the standard Concrete implementation
pub fn view_to_concrete<T, TN, CN, Val>(t: T) -> Concrete<TN, CN, Val>
where
    T: View<Val = Val, TN = TN, CN = CN>,
    Val: Clone,
    TN: Clone,
    CN: Clone + Eq + Hash,
{
    type Copier<TN, CN, Val> = Option<Concrete<TN, CN, Val>>;

    ImplBase!(Copier<TN, CN, Val>);

    impl<TN, CN, Val> Visitor for Copier<TN, CN, Val>
    where
        Val: Clone,
        TN: Clone,
        CN: Clone + Eq + Hash,
    {
        fn visit_map<T: MapView<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &mut self,
            type_name: &Self::TN,
            t: &T,
        ) {
            *self = Some(Concrete {
                type_name: type_name.clone(),
                content: StructOrValue::Struct(t.apply(HashMap::new())),
            });
        }
        fn visit_value(&mut self, type_name: &Self::TN, t: &Vec<Self::Val>) {
            *self = Some(Concrete {
                type_name: type_name.clone(),
                content: StructOrValue::Val(t.clone()),
            })
        }
    }

    impl<TN, CN, Val> MapVisitor for HashMap<CN, Vec<Concrete<TN, CN, Val>>>
    where
        Val: Clone,
        TN: Clone,
        CN: Clone + Eq + Hash,
    {
        fn visit<T: ListView<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &mut self,
            name: &Self::CN,
            children: &T,
        ) {
            self.insert(name.clone(), children.apply(vec![]));
        }
    }

    impl<TN, CN, Val> ListVisitor for Vec<Concrete<TN, CN, Val>>
    where
        Val: Clone,
        TN: Clone,
        CN: Clone + Eq + Hash,
    {
        fn visit<T: View<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&mut self, child: &T) {
            self.push(child.apply(None).unwrap());
        }
    }

    return t.apply(None).unwrap();
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
