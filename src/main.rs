#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate byteorder;
extern crate serde;
extern crate serde_cbor;
extern crate serde_json;

#[macro_use]
mod data_models;

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

    data_models::typed_value_tree::view_to_concrete(data);

    // println!(
    //     "reflective = {}",
    //     self_describing_tree_view::dump(data).len() as f64 / bin_code_size
    // );
}

pub mod type_to_leaf {
    use super::data_models::leaf_tree::{View, Visitor};
    use super::data_models::typed_value_tree::{
        Base, ListView, ListVisitor, MapView, MapVisitor, TypeView, TypeVisitor,
    };

    struct TypeToLeafTag {}

    impl<T> View<TypeToLeafTag> for T
    where
        T: TypeView<TypeToLeafTag, TN = u128, CN = u128, Val = u8>,
    {
        type Value = u8;
        fn visit<V: Visitor<TypeToLeafTag, Value = u8>>(&self, v: &mut V) {
            //self.decoder.visit_root(&self.data, v);
        }
    }
}

// Design TODO:
// Consider ways to lifetime extend View_s to enable incremental/lazy traversal and/or references to locations in trees

/// Relating to how a leaf tree is formatted within a byte sequence.
/// Does not implement any encodings, just declare the traits encoders and decoders will implement.
pub mod encoding {
    use super::data_models::leaf_tree::{View, Visitor};

    struct EncodeTag {}

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
        fn visit_root<V: Visitor<EncodeTag, Value = Self::Value>>(&self, data: &Vec<u8>, v: &mut V);
    }

    // Implement this to define a way to serialize leaf trees.
    pub trait Encoder {
        type Value;
        fn serialize<TView: View<EncodeTag, Value = Self::Value>>(&self, v: &TView) -> Vec<u8>;
    }

    impl<TDecoder, Value> View<EncodeTag> for EncodedLeafTree<TDecoder, Value>
    where
        TDecoder: Decoder<Value = Value>,
    {
        type Value = Value;
        fn visit<V: Visitor<EncodeTag, Value = Value>>(&self, v: &mut V) {
            self.decoder.visit_root(&self.data, v);
        }
    }
}

mod test_data {
    use super::data_models::leaf_tree::View as Tree;
    use super::data_models::leaf_tree::Visitor;
    use super::self_describing_tree_view::{Id, Named};

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

    // impl Tree for TestData {
    //     type Value = u8;
    //     fn visit<V: Visitor<Value = Self::Value>>(&self, v: &mut V) {
    //         for color in &self.colors {
    //             v.visit_list(color);
    //         }
    //     }
    // }

    // impl Tree for Color {
    //     type Value = u8;
    //     fn visit<V: Visitor<Value = Self::Value>>(&self, v: &mut V) {
    //         visit_field(v, &self.r, Id { id: 7383786837 });
    //         visit_field(v, &self.g, Id { id: 4525787583 });
    //         visit_field(v, &self.b, Id { id: 3787388378 });
    //         visit_field(v, &self.a, Id { id: 7837387833 });
    //     }
    // }

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

mod self_describing_tree_view2 {
    use super::data_models::typed_value_tree::{
        Base, ListView, ListVisitor, MapView, MapVisitor, TypeView, TypeVisitor,
    };
    use super::self_describing_tree_view::Named;
    use super::test_data::{Color, TestData};

    pub struct Tag {}

    impl TypeView<Tag> for TestData {
        type TN = u128;
        type CN = u128;
        type Val = u8;

        fn visit<V: TypeVisitor<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &self,
            v: &mut V,
        ) {
            v.visit_map(&Self::get_id().id, self);
        }
    }

    impl MapView<Tag> for TestData {
        type TN = u128;
        type CN = u128;
        type Val = u8;

        fn visit<V: MapVisitor<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &self,
            v: &mut V,
        ) {
            v.visit(&1234u128, &self.colors);
        }
    }

    impl ListView<Tag> for Vec<Color> {
        type TN = u128;
        type CN = u128;
        type Val = u8;

        fn visit<V: ListVisitor<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &self,
            v: &mut V,
        ) {
            for child in self.iter() {
                v.visit(child);
            }
        }
    }

    impl ListView<Tag> for u8 {
        type TN = u128;
        type CN = u128;
        type Val = u8;

        fn visit<V: ListVisitor<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &self,
            v: &mut V,
        ) {
            v.visit(self);
        }
    }

    impl TypeView<Tag> for Color {
        type TN = u128;
        type CN = u128;
        type Val = u8;

        fn visit<V: TypeVisitor<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &self,
            v: &mut V,
        ) {
            v.visit_map(&Self::get_id().id, self);
        }
    }

    impl MapView<Tag> for Color {
        type TN = u128;
        type CN = u128;
        type Val = u8;

        fn visit<V: MapVisitor<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &self,
            v: &mut V,
        ) {
            v.visit(&1255454u128, &self.r);
            v.visit(&1215334u128, &self.g);
            v.visit(&1213534u128, &self.b);
            v.visit(&1231354u128, &self.a);
        }
    }

    impl TypeView<Tag> for u8 {
        type TN = u128;
        type CN = u128;
        type Val = u8;

        fn visit<V: TypeVisitor<Tag, TN = Self::TN, CN = Self::CN, Val = Self::Val>>(
            &self,
            v: &mut V,
        ) {
            v.visit_value(&Self::get_id().id, &vec![*self]);
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

    // pub fn dump<T>(t: T) -> Vec<u8>
    // where
    //     T: Tree<Value = u8> + Named,
    // {
    //     struct Dumper {
    //         out: Vec<u8>,
    //     }

    //     impl Visitor for Dumper {
    //         type Value = u8;
    //         fn visit_list<T: Tree<Value = u8>>(&mut self, t: &T) {
    //             t.visit::<Dumper>(self);
    //         }
    //         fn visit_value(&mut self, t: Self::Value) {
    //             self.out.push(t);
    //         }
    //     }

    //     let mut d = Dumper { out: vec![] };
    //     visit_typed_value(&mut d, &t);
    //     return d.out;
    // }

    // impl Tree for Id {
    //     type Value = u8;

    //     fn visit<V: Visitor<Value = Self::Value>>(&self, v: &mut V) {
    //         let mut wtr = vec![];
    //         wtr.write_u128::<byteorder::LittleEndian>(self.id).unwrap();
    //         for u in wtr {
    //             v.visit_value(u);
    //         }
    //     }
    // }

    impl Named for u8 {
        fn get_id() -> Id {
            Id { id: 368573854 }
        }
    }

    // pub fn visit_field<'a, T, V: Visitor<Value = u8>>(v: &mut V, field: &'a T, id: Id)
    // where
    //     &'a T: Tree<Value = u8>,
    //     T: Tree<Value = u8> + Named, // TODO: why are both needed?
    // {
    //     struct Field<'a, T>
    //     where
    //         &'a T: Tree<Value = u8>,
    //         T: Named,
    //     {
    //         id: Id,
    //         f: &'a T,
    //     }

    //     impl<'a, T> Tree for Field<'a, T>
    //     where
    //         T: Tree<Value = u8> + Named,
    //         &'a T: Tree<Value = u8>,
    //     {
    //         type Value = u8;

    //         fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
    //             v.visit_list(&self.id);
    //             // nest f and its type name under another node
    //             visit_typed_value(v, &self.f);
    //         }
    //     }

    //     let f = Field::<'a, T> { id: id, f: field };
    //     v.visit_list(&f);
    // }

    // // TODO: make this the impl for T, and make a separate helper for content
    // fn visit_typed_value<'a, T, V: Visitor<Value = u8>>(v: &mut V, value: &'a T)
    // where
    //     &'a T: Tree<Value = u8>,
    //     T: Tree<Value = u8> + Named, // TODO: why are both needed?
    // {
    //     struct NamedValue<'a, T>
    //     where
    //         &'a T: Tree<Value = u8>,
    //         T: Named,
    //     {
    //         f: &'a T,
    //     }

    //     impl<'a, T> Tree for NamedValue<'a, T>
    //     where
    //         T: Tree<Value = u8> + Named,
    //         &'a T: Tree<Value = u8>,
    //     {
    //         type Value = u8;

    //         fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
    //             v.visit_list(&T::get_id());
    //             v.visit_list(&self.f);
    //         }
    //     }

    //     let f = NamedValue::<'a, T> { f: value };
    //     v.visit_list(&f);
    // }

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

    // impl Tree for u8 {
    //     type Value = u8;
    //     fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
    //         v.visit_value(*self);
    //     }
    // }

    // // TODO: why do we need this?
    // impl<T> Tree for &T
    // where
    //     T: Tree<Value = u8>,
    // {
    //     type Value = u8;
    //     fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
    //         (*self).visit(v);
    //     }
    // }
}
