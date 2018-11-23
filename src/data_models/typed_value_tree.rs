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

pub struct Concrete<TN, CN, Val> {
    type_name: TN,
    content: StructOrValue<TN, CN, Val>,
}

/// Content helper for Concrete.
pub enum StructOrValue<TN, CN, Val> {
    Struct(HashMap<CN, Vec<Concrete<TN, CN, Val>>>),
    Val(Vec<Val>),
}

pub trait View {
    type TN;
    type CN;
    type Val;
    fn visit<V: Visitor<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&self, v: &mut V);
}

pub trait MapView {
    type TN;
    type CN;
    type Val;
    fn visit<V: MapVisitor<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&self, v: &mut V);
}

pub trait ListView {
    type TN;
    type CN;
    type Val;
    fn visit<V: ListVisitor<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&self, v: &mut V);
}

pub trait Visitor {
    type TN;
    type CN;
    type Val;

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

pub trait MapVisitor {
    type TN;
    type CN;
    type Val;

    /// Called for with value in the map
    fn visit<T: ListView<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
        &mut self,
        name: &Self::CN,
        children: &T,
    );
}

pub trait ListVisitor {
    type TN;
    type CN;
    type Val;

    /// Called for with value in the children list in order
    fn visit<T: View<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&mut self, child: &T);
}

impl<TN, CN, Val> View for Concrete<TN, CN, Val>
where
    Val: Clone,
    CN: Eq + Hash,
{
    type TN = TN;
    type CN = CN;
    type Val = Val;

    fn visit<V: Visitor<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&self, v: &mut V) {
        match &self.content {
            StructOrValue::Struct(map) => {
                v.visit_map(&self.type_name, map);
            }
            StructOrValue::Val(value) => {
                v.visit_value(&self.type_name, value);
            }
        };
    }
}

impl<Val, TN, CN> MapView for HashMap<CN, Vec<Concrete<TN, CN, Val>>>
where
    Val: Clone,
    CN: Eq + Hash,
{
    type TN = TN;
    type CN = CN;
    type Val = Val;

    fn visit<V: MapVisitor<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&self, v: &mut V) {
        for (k, children) in self.iter() {
            v.visit(&k, children);
        }
    }
}

impl<Val, TN, CN> ListView for Vec<Concrete<TN, CN, Val>>
where
    Val: Clone,
    CN: Eq + Hash,
{
    type TN = TN;
    type CN = CN;
    type Val = Val;

    fn visit<V: ListVisitor<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&self, v: &mut V) {
        for child in self.iter() {
            v.visit(child);
        }
    }
}

// Copy into the standard Concrete implementation
pub fn view_to_concrete<T, TN, CN, Val>(t: T) -> Concrete<TN, CN, Val>
where
    T: View<Val = Val, TN = TN, CN = CN>,
    Val: Clone,
    TN: Clone,
    CN: Clone + Eq + Hash,
{
    struct Copier<TN, CN, Val> {
        out: Option<Concrete<TN, CN, Val>>,
    }

    impl<TN, CN, Val> Visitor for Copier<TN, CN, Val>
    where
        Val: Clone,
        TN: Clone,
        CN: Clone + Eq + Hash,
    {
        type TN = TN;
        type CN = CN;
        type Val = Val;
        fn visit_map<T: MapView<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &mut self,
            type_name: &Self::TN,
            t: &T,
        ) {
            let mut map = HashMap::new();
            t.visit(&mut map);
            self.out = Some(Concrete {
                type_name: type_name.clone(),
                content: StructOrValue::Struct(map),
            });
        }
        fn visit_value(&mut self, type_name: &Self::TN, t: &Vec<Self::Val>) {
            self.out = Some(Concrete {
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
        type TN = TN;
        type CN = CN;
        type Val = Val;
        fn visit<T: ListView<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &mut self,
            name: &Self::CN,
            children: &T,
        ) {
            let mut out: Vec<Concrete<TN, CN, Val>> = vec![];
            children.visit(&mut out);
            self.insert(name.clone(), out);
        }
    }

    impl<TN, CN, Val> ListVisitor for Vec<Concrete<TN, CN, Val>>
    where
        Val: Clone,
        TN: Clone,
        CN: Clone + Eq + Hash,
    {
        type TN = TN;
        type CN = CN;
        type Val = Val;
        fn visit<T: View<TN = Self::TN, CN = Self::CN, Val = Self::Val>>(&mut self, child: &T) {
            let mut d = Copier { out: None };
            child.visit(&mut d);
            self.push(d.out.unwrap());
        }
    }

    let mut d = Copier { out: None };
    t.visit(&mut d);
    return d.out.into_iter().nth(0).unwrap();
}
