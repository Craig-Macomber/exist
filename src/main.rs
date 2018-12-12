pub mod basic_encoding;
pub mod data_models;
pub mod prefix_encoding;
pub mod type_to_leaf;

use self::encoding::*;
use self::prefix_encoding::PrefixCompressedEncoding;
use self::type_to_leaf::TypeViewer;

fn main() {
    let mut colors = vec![];
    for i in 0..400 {
        colors.push(test_data::Color {
            r: (i % 256) as u8,
            g: (i / 256 % 256) as u8,
            b: (i / 256 / 256 % 256) as u8,
            a: 255,
        });
    }

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
    data_models::leaf_tree::concrete::view_to_concrete(&TypeViewer(&data));

    let encoded = PrefixCompressedEncoding.serialize(&TypeViewer(&data));
    println!("exist = {}", encoded.len() as f64 / bin_code_size);

    let decoded_view = EncodedLeafTree {
        decoder: PrefixCompressedEncoding,
        data: encoded,
    };

    data_models::leaf_tree::concrete::view_to_concrete(&decoded_view);
    let encoded2 = PrefixCompressedEncoding.serialize(&decoded_view);
    assert_eq!(&decoded_view.data, &encoded2);
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
        pub decoder: TDecoder,
        pub data: Vec<u8>,
    }

    // Implement this to define a way to deserialize leaf trees.
    pub trait Decoder {
        type Value;
        fn visit_root<V: Visitor<Value = Self::Value>>(&self, data: &[u8], v: &mut V);
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

    #[cfg(test)]
    mod tests {
        use super::super::basic_encoding::BasicEncoding;
        use super::super::data_models::leaf_tree::concrete::{view_to_concrete, Concrete};
        use super::super::prefix_encoding::{PrefixCompressedEncoding, PrefixEncoding};
        use super::*;

        fn encode_round_trip<T: Encoder<Value = u8> + Decoder<Value = u8>>(c: &Concrete<u8>, e: T) {
            let input_copy = view_to_concrete(c);
            assert_eq!(&input_copy, c, "copy");

            let encoded = e.serialize(c);
            let decoded_view = EncodedLeafTree {
                decoder: e,
                data: encoded,
            };
            let decoded_copy = view_to_concrete(&decoded_view);
            assert_eq!(&decoded_copy, c, "decode");
        }

        fn check(c: Concrete<u8>, v: Vec<u8>) {
            encode_round_trip(&c, BasicEncoding);
            let encoded = BasicEncoding.serialize(&c);
            assert_eq!(encoded, v, "encode");
            check2(c);
        }

        fn check2(c: Concrete<u8>) {
            encode_round_trip(&c, BasicEncoding);
            encode_round_trip(&c, PrefixEncoding);
            encode_round_trip(&c, PrefixCompressedEncoding);
        }

        #[test]
        fn encode_empty() {
            check(Concrete::List(vec![]), Vec::<u8>::new());
        }

        #[test]
        fn encode_value() {
            check(Concrete::Value(12), vec![1, 12]);
        }

        #[test]
        fn encode_list() {
            check(Concrete::List(vec![Concrete::Value(12)]), vec![0, 1, 12, 2]);
        }

        #[test]
        fn encode_list2() {
            check(
                Concrete::List(vec![Concrete::Value(12), Concrete::Value(13)]),
                vec![0, 1, 12, 2, 0, 1, 13, 2],
            );
        }

        #[test]
        fn encode_list_dup() {
            check2(Concrete::List(vec![
                Concrete::List(vec![Concrete::Value(12)]),
                Concrete::List(vec![Concrete::Value(12)]),
                Concrete::List(vec![Concrete::Value(12)]),
                Concrete::List(vec![Concrete::Value(12)]),
            ]));
        }

        #[test]
        fn encode_list_dup2() {
            check2(Concrete::List(vec![
                Concrete::List(vec![Concrete::Value(12)]),
                Concrete::List(vec![Concrete::Value(12), Concrete::Value(12)]),
                Concrete::List(vec![Concrete::Value(12)]),
                Concrete::List(vec![Concrete::Value(13)]),
            ]));
        }
    }
}

#[macro_use]
mod into_typed_value_tree {
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
}

mod test_data {
    use super::data_models::typed_value_tree::{MapView, MapVisitor, TypeView, TypeVisitor};
    use super::into_typed_value_tree::{
        visit_list_field, visit_single_field, Named, Struct, Terminal,
    };
    use serde_derive::{Deserialize, Serialize};

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

    TypeViewForTerminal!(u8);
    impl Terminal for u8 {
        fn get_id() -> u128 {
            3
        }

        fn bytes(&self) -> Vec<u8> {
            vec![*self]
        }
    }

    TypeViewForStruct!(TestData);
    impl Struct for TestData {
        fn get_id() -> u128 {
            1
        }

        fn visit<V: MapVisitor<N = u128>>(&self, v: &mut V) {
            visit_list_field(v, &1234u128, &self.colors);
        }
    }

    TypeViewForStruct!(Color);
    impl Struct for Color {
        fn get_id() -> u128 {
            2
        }

        fn visit<V: MapVisitor<N = u128>>(&self, v: &mut V) {
            visit_single_field(v, &1255454u128, &self.r);
            visit_single_field(v, &1215334u128, &self.g);
            visit_single_field(v, &1213534u128, &self.b);
            visit_single_field(v, &1231354u128, &self.a);
        }
    }
}
