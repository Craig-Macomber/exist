#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate byteorder;
extern crate serde;
extern crate serde_cbor;
extern crate serde_json;

fn main() {
    let colors = vec![
        test_data::Color {
            r: 123,
            g: 200,
            b: 153,
            a: 255,
        };
        1000
    ];
    let data = test_data::TestData { colors };

    let bin_code_size = bincode::serialize(&data).unwrap().len() as f64;
    println!("Overhead relative to bincode:");
    println!(
        "json = {}",
        serde_json::to_string(&data).unwrap().len() as f64 / bin_code_size
    );
    println!(
        "serde_cbor = {}",
        serde_cbor::to_vec(&data).unwrap().len() as f64 / bin_code_size
    );
    println!(
        "reflective = {}",
        self_describing_tree_view::dump(data).len() as f64 / bin_code_size
    );
}

/// Data Models used in this library.
/// Since there is no single Trait that an object will implement to expose itself as one of these data models,
/// the names for the models are used by nested modules.
/// Each nested module contains a reference concrete implementations with minimal complexity
/// as well as traits for generic implementations/adapters based on the visitor pattern.
///
/// These modules do not depend on each other, and adapters/converts are separate.
pub mod data_models {
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

        pub trait View {
            type Value;
            fn visit<V: Visitor<Value = Self::Value>>(&self, v: &mut V);
        }

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
            struct Copier<V> {
                out: Vec<Concrete<V>>,
            }

            impl<V> Visitor for Copier<V> {
                type Value = V;
                fn visit_list<T: View<Value = V>>(&mut self, t: &T) {
                    let mut d = Copier { out: vec![] };
                    t.visit::<Copier<V>>(&mut d);
                    self.out.push(Concrete::List(d.out));
                }
                fn visit_value(&mut self, v: V) {
                    self.out.push(Concrete::Value(v));
                }
            }

            let mut d = Copier { out: vec![] };
            t.visit(&mut d);
            return d.out.into_iter().nth(0).unwrap();
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

    /// TypedTree, but can store value direct in the nodes instead of just children.
    pub mod typed_value_tree {
        use std::collections::HashMap;
        use std::hash::Hash;

        pub struct Concrete<TypeName, ChildName, V> {
            type_name: TypeName,
            content: StructOrValue<TypeName, ChildName, V>,
        }

        /// Content helper for Concrete.
        pub enum StructOrValue<TypeName, ChildName, V> {
            Struct(HashMap<ChildName, Vec<Concrete<TypeName, ChildName, V>>>),
            Value(Vec<V>),
        }

        pub trait View {
            type Value;
            type TypeName;
            type ChildName;

            fn visit<
                V: Visitor<
                    Value = Self::Value,
                    TypeName = Self::TypeName,
                    ChildName = Self::ChildName,
                >,
            >(
                &self,
                v: &mut V,
            );
        }

        pub trait MapView {
            type Value;
            type TypeName;
            type ChildName;
            fn visit<
                V: MapVisitor<
                    Value = Self::Value,
                    TypeName = Self::TypeName,
                    ChildName = Self::ChildName,
                >,
            >(
                &self,
                v: &mut V,
            );
        }

        pub trait ListView {
            type Value;
            type TypeName;
            type ChildName;
            fn visit<
                V: ListVisitor<
                    Value = Self::Value,
                    TypeName = Self::TypeName,
                    ChildName = Self::ChildName,
                >,
            >(
                &self,
                v: &mut V,
            );
        }

        pub trait Visitor {
            type Value;
            type TypeName;
            type ChildName;

            /// Called if the View was a Struct (map)
            fn visit_map<
                T: MapView<
                    Value = Self::Value,
                    TypeName = Self::TypeName,
                    ChildName = Self::ChildName,
                >,
            >(
                &mut self,
                type_name: &Self::TypeName,
                t: &T,
            );

            /// Called if the View was a Value / Leaf
            // TODO: use ExactSizeIterator?
            fn visit_value(&mut self, type_name: &Self::TypeName, t: &Vec<Self::Value>);
        }

        pub trait MapVisitor {
            type Value;
            type TypeName;
            type ChildName;

            /// Called for with value in the map
            fn visit<T: ListView<Value = Self::Value>>(
                &mut self,
                name: &Self::ChildName,
                children: &T,
            );
        }

        pub trait ListVisitor {
            type Value;
            type TypeName;
            type ChildName;

            /// Called for with value in the children list in order
            fn visit<
                T: View<Value = Self::Value, TypeName = Self::TypeName, ChildName = Self::ChildName>,
            >(
                &mut self,
                child: &T,
            );
        }

        impl<Value, TypeName, ChildName> View for Concrete<TypeName, ChildName, Value>
        where
            Value: Clone,
            ChildName: Eq + Hash,
        {
            type Value = Value;
            type TypeName = TypeName;
            type ChildName = ChildName;

            fn visit<
                V: Visitor<
                    Value = Self::Value,
                    TypeName = Self::TypeName,
                    ChildName = Self::ChildName,
                >,
            >(
                &self,
                v: &mut V,
            ) {
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

        impl<Value, TypeName, ChildName> MapView
            for HashMap<ChildName, Vec<Concrete<TypeName, ChildName, Value>>>
        where
            Value: Clone,
            ChildName: Eq + Hash,
        {
            type Value = Value;
            type TypeName = TypeName;
            type ChildName = ChildName;

            fn visit<
                V: MapVisitor<
                    Value = Self::Value,
                    TypeName = Self::TypeName,
                    ChildName = Self::ChildName,
                >,
            >(
                &self,
                v: &mut V,
            ) {
                for (k, children) in self.iter() {
                    v.visit(&k, children);
                }
            }
        }

        impl<Value, TypeName, ChildName> ListView for Vec<Concrete<TypeName, ChildName, Value>>
        where
            Value: Clone,
            ChildName: Eq + Hash,
        {
            type Value = Value;
            type TypeName = TypeName;
            type ChildName = ChildName;

            fn visit<
                V: ListVisitor<
                    Value = Self::Value,
                    TypeName = Self::TypeName,
                    ChildName = Self::ChildName,
                >,
            >(
                &self,
                v: &mut V,
            ) {
                for child in self.iter() {
                    v.visit(child);
                }
            }
        }
    }
}

// Design TODO:
// Consider ways to lifetime extend View_s to enable incremental/lazy traversal and/or references to locations in trees

/// Relating to how a leaf tree is formatted within a byte sequence.
/// Does not implement any encodings, just declare the traits encoders and decoders will implement.
pub mod encoding {
    use super::data_models::leaf_tree::{View, Visitor};

    pub struct EncodedLeafTree<TDecoder, Value>
    where
        TDecoder: Decoder<Value = Value>,
    {
        decoder: TDecoder,
        data: Vec<u8>,
    }

    // Implement this to define a way to deserialize leaf trees.
    pub trait Decoder {
        type Value;
        fn visit_root<V: Visitor<Value = Self::Value>>(&self, data: &Vec<u8>, v: &mut V);
    }

    // Implement this to define a way to serialize leaf trees.
    pub trait Encoder {
        type Value;
        fn serialize<TView: View<Value = Self::Value>>(&self, v: &TView) -> Vec<u8>;
    }

    impl<TDecoder, Value> View for EncodedLeafTree<TDecoder, Value>
    where
        TDecoder: Decoder<Value = Value>,
    {
        type Value = Value;
        fn visit<V: Visitor<Value = Value>>(&self, v: &mut V) {
            self.decoder.visit_root(&self.data, v);
        }
    }
}

mod test_data {
    use super::data_models::leaf_tree::View as Tree;
    use super::data_models::leaf_tree::Visitor;
    use super::self_describing_tree_view::{visit_field, Id, Named};

    #[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
    pub struct TestData {
        #[serde(rename = "5c93cbae1acd44b58223c1fdb91fa475")]
        pub colors: Vec<Color>,
    }

    #[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
    pub struct Color {
        #[serde(rename = "436ff18bf3f14263856343a575edd1c6")]
        pub r: u8,
        #[serde(rename = "924dfcb72032475192cdc78dc8a7d8ca")]
        pub g: u8,
        #[serde(rename = "b166467549754a73b8e19f98446abb5c")]
        pub b: u8,
        #[serde(rename = "4a7980316b8740ada6f46d6f50009e2b")]
        pub a: u8,
    }

    impl Tree for TestData {
        type Value = u8;
        fn visit<V: Visitor<Value = Self::Value>>(&self, v: &mut V) {
            for color in &self.colors {
                v.visit_list(color);
            }
        }
    }

    impl Tree for Color {
        type Value = u8;
        fn visit<V: Visitor<Value = Self::Value>>(&self, v: &mut V) {
            visit_field(v, &self.r, Id { id: 7383786837 });
            visit_field(v, &self.g, Id { id: 4525787583 });
            visit_field(v, &self.b, Id { id: 3787388378 });
            visit_field(v, &self.a, Id { id: 7837387833 });
        }
    }

    impl Named for Color {
        fn get_id() -> Id {
            Id { id: 2 }
        }
    }

    impl Named for TestData {
        fn get_id() -> Id {
            Id { id: 1 }
        }
    }
}

// Tools for making types viewable as leaf_tree::View<Value = u8> suitable for persistance as self describing data
mod self_describing_tree_view {
    use super::data_models::leaf_tree::View as Tree;
    use super::data_models::leaf_tree::Visitor;
    use byteorder::WriteBytesExt;

    // A Field ID or Type ID
    #[derive(Debug, Eq, PartialEq, Clone)]
    pub struct Id {
        pub id: u128,
    }

    pub trait Named {
        fn get_id() -> Id;
    }

    pub fn dump<T>(t: T) -> Vec<u8>
    where
        T: Tree<Value = u8> + Named,
    {
        struct Dumper {
            out: Vec<u8>,
        }

        impl Visitor for Dumper {
            type Value = u8;
            fn visit_list<T: Tree<Value = u8>>(&mut self, t: &T) {
                t.visit::<Dumper>(self);
            }
            fn visit_value(&mut self, t: Self::Value) {
                self.out.push(t);
            }
        }

        let mut d = Dumper { out: vec![] };
        visit_typed_value(&mut d, &t);
        return d.out;
    }

    impl Tree for Id {
        type Value = u8;

        fn visit<V: Visitor<Value = Self::Value>>(&self, v: &mut V) {
            let mut wtr = vec![];
            wtr.write_u128::<byteorder::LittleEndian>(self.id).unwrap();
            for u in wtr {
                v.visit_value(u);
            }
        }
    }

    impl Named for u8 {
        fn get_id() -> Id {
            Id { id: 368573854 }
        }
    }

    pub fn visit_field<'a, T, V: Visitor<Value = u8>>(v: &mut V, field: &'a T, id: Id)
    where
        &'a T: Tree<Value = u8>,
        T: Tree<Value = u8> + Named, // TODO: why are both needed?
    {
        struct Field<'a, T>
        where
            &'a T: Tree<Value = u8>,
            T: Named,
        {
            id: Id,
            f: &'a T,
        }

        impl<'a, T> Tree for Field<'a, T>
        where
            T: Tree<Value = u8> + Named,
            &'a T: Tree<Value = u8>,
        {
            type Value = u8;

            fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
                v.visit_list(&self.id);
                // nest f and its type name under another node
                visit_typed_value(v, &self.f);
            }
        }

        let f = Field::<'a, T> { id: id, f: field };
        v.visit_list(&f);
    }

    // TODO: make this the impl for T, and make a separate helper for content
    fn visit_typed_value<'a, T, V: Visitor<Value = u8>>(v: &mut V, value: &'a T)
    where
        &'a T: Tree<Value = u8>,
        T: Tree<Value = u8> + Named, // TODO: why are both needed?
    {
        struct NamedValue<'a, T>
        where
            &'a T: Tree<Value = u8>,
            T: Named,
        {
            f: &'a T,
        }

        impl<'a, T> Tree for NamedValue<'a, T>
        where
            T: Tree<Value = u8> + Named,
            &'a T: Tree<Value = u8>,
        {
            type Value = u8;

            fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
                v.visit_list(&T::get_id());
                v.visit_list(&self.f);
            }
        }

        let f = NamedValue::<'a, T> { f: value };
        v.visit_list(&f);
    }

    // trait StructType: Named {
    //     type ItemType;
    // }

    // trait PrimativeType: Named {
    //     type ItemType;
    // }

    // trait SequenceType: Named {
    //     type ItemType;
    // }

    // trait Type: Named {
    //     type ItemType;
    // }

    // // Data Models:
    // // Storage:
    // //  Essentially a LeafTree<u8>, encoded
    // // All data is either:
    // // - Struct

    // impl<T> Type for T where T: StructType {}
    // impl<T> Type for T where T: PrimativeType {}

    // trait TypedValue: Type {}

    // trait Sequence {
    //     type ItemType;
    // }

    // impl<T> Tree for T
    // where
    //     T: Type,
    // {
    //     type Value = u8;
    //     fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
    //         v.visit_value(*self);
    //     }
    // }

    impl Tree for u8 {
        type Value = u8;
        fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
            v.visit_value(*self);
        }
    }

    // TODO: why do we need this?
    impl<T> Tree for &T
    where
        T: Tree<Value = u8>,
    {
        type Value = u8;
        fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
            (*self).visit(v);
        }
    }
}
