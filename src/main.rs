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

    data_models::typed_value_tree::concrete::view_to_concrete(&data);
    data_models::leaf_tree::concrete::view_to_concrete(&data);

    // println!(
    //     "exist = {}",
    //     self_describing_tree_view::dump(data).len() as f64 / bin_code_size
    // );
}

/// Output tree looks like:
/// TypedValue (List)
///     TypeName (List)
///         first byte (Value)
///         ...
///         16th byte (Value)
///     Content (List)
///         ...
///
/// If Content is Value:
/// Content (List)
///     first byte (Value)
///     ...
///     last byte (Value)
///
/// Content can have 0+ bytes
///
/// If Content is Struct/Map:
/// Content (List)
///     Map Marker (List) (needed so we can tell empty map from empty Value)
///         first map entry (List)
///             ChildName (List)
///                 first byte (Value)
///                 ...
///                 16th byte (Value)
///             Children List (List)
///                 TypedValue (List) (structure recurses here)
///
/// Content can have 0+ map entries
pub mod type_to_leaf {
    use super::data_models::leaf_tree::{View, Visitor};
    use super::data_models::typed_value_tree::{
        ListView, ListVisitor, MapView, MapVisitor, TypeView, TypeVisitor,
    };
    use byteorder::WriteBytesExt;

    struct ByteLister {
        v: Vec<u8>,
    }

    fn make_byte_lister(n: u128) -> ByteLister {
        let mut wtr = vec![];
        wtr.write_u128::<byteorder::LittleEndian>(n).unwrap();
        return ByteLister { v: wtr };
    }

    impl View for ByteLister {
        type Value = u8;
        fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
            for u in self.v.iter() {
                v.visit_value(*u);
            }
        }
    }

    impl<T> View for T
    where
        T: TypeView<N = u128>,
    {
        type Value = u8;
        fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
            let type_name = self.apply(TypeGetter(0u128));

            // Type: list of bytes containing type's id
            v.visit_list(&make_byte_lister(type_name.0));

            // Content: List of map entries OR List of bytes if terminal type
            v.visit_list(&ContentLister(self));

            struct TypeGetter(u128);
            impl TypeVisitor for TypeGetter {
                type N = u128;

                fn visit_map<T: MapView<N = Self::N>>(&mut self, type_name: &Self::N, _t: &T) {
                    self.0 = *type_name;
                }
                fn visit_value(&mut self, type_name: &Self::N, _t: &Vec<u8>) {
                    self.0 = *type_name;
                }
            }
        }
    }

    struct ContentLister<T>(T);
    impl<T> View for ContentLister<&T>
    where
        T: TypeView<N = u128>,
    {
        type Value = u8;
        fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
            self.0.visit(&mut ContentListerVisiter(v));

            struct ContentListerVisiter<V>(V);
            impl<V> TypeVisitor for ContentListerVisiter<&mut V>
            where
                V: Visitor<Value = u8>,
            {
                type N = u128;

                fn visit_map<T: MapView<N = Self::N>>(&mut self, _type_name: &Self::N, t: &T) {
                    // Map Marker
                    self.0.visit_list(&MapLister(t));
                }

                fn visit_value(&mut self, _type_name: &Self::N, t: &Vec<u8>) {
                    // Content:
                    // List of bytes for terminal type
                    self.0.visit_list(&ByteLister { v: t.clone() });
                }
            }
        }
    }

    struct MapLister<T>(T);
    impl<T> View for MapLister<&T>
    where
        T: MapView<N = u128>,
    {
        type Value = u8;
        fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
            self.0.visit(&mut MapListerVisiter(v));

            struct MapListerVisiter<V>(V);
            impl<V> MapVisitor for MapListerVisiter<&mut V>
            where
                V: Visitor<Value = u8>,
            {
                type N = u128;

                fn visit<T: ListView<N = Self::N>>(&mut self, name: &Self::N, children: &T) {
                    // Child Name / Map Key / Field Name: list of bytes containing name id
                    self.0.visit_list(&make_byte_lister(*name));

                    // Children List
                    self.0.visit_list(&ChildLister(children));
                }
            }
        }
    }

    struct ChildLister<T>(T);
    impl<T> View for ChildLister<&T>
    where
        T: ListView<N = u128>,
    {
        type Value = u8;
        fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
            self.0.visit(&mut ChildListerVisiter(v));

            struct ChildListerVisiter<V>(V);
            impl<V> ListVisitor for ChildListerVisiter<&mut V>
            where
                V: Visitor<Value = u8>,
            {
                type N = u128;

                fn visit<T: TypeView<N = Self::N>>(&mut self, child: &T) {
                    // TypedValue
                    self.0.visit_list(child);
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
    use super::into_typed_value_tree::Named;

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
        fn get_id() -> u128 {
            2
        }
    }

    impl Named for TestData {
        fn get_id() -> u128 {
            1
        }
    }

    impl Named for u8 {
        fn get_id() -> u128 {
            3
        }
    }
}

mod into_typed_value_tree {
    use super::data_models::typed_value_tree::{
        ListView, ListVisitor, MapView, MapVisitor, TypeView, TypeVisitor,
    };

    use super::test_data::{Color, TestData};

    pub trait Named {
        fn get_id() -> u128;
    }

    impl TypeView for TestData {
        type N = u128;

        fn visit<V: TypeVisitor<N = Self::N>>(&self, v: &mut V) {
            v.visit_map(&Self::get_id(), self);
        }
    }

    impl MapView for TestData {
        type N = u128;

        fn visit<V: MapVisitor<N = Self::N>>(&self, v: &mut V) {
            v.visit(&1234u128, &self.colors);
        }
    }

    impl ListView for Vec<Color> {
        type N = u128;

        fn visit<V: ListVisitor<N = Self::N>>(&self, v: &mut V) {
            for child in self.iter() {
                v.visit(child);
            }
        }
    }

    impl ListView for u8 {
        type N = u128;

        fn visit<V: ListVisitor<N = Self::N>>(&self, v: &mut V) {
            v.visit(self);
        }
    }

    impl TypeView for Color {
        type N = u128;

        fn visit<V: TypeVisitor<N = Self::N>>(&self, v: &mut V) {
            v.visit_map(&Self::get_id(), self);
        }
    }

    impl MapView for Color {
        type N = u128;

        fn visit<V: MapVisitor<N = Self::N>>(&self, v: &mut V) {
            v.visit(&1255454u128, &self.r);
            v.visit(&1215334u128, &self.g);
            v.visit(&1213534u128, &self.b);
            v.visit(&1231354u128, &self.a);
        }
    }

    impl TypeView for u8 {
        type N = u128;

        fn visit<V: TypeVisitor<N = Self::N>>(&self, v: &mut V) {
            v.visit_value(&Self::get_id(), &vec![*self]);
        }
    }
}
