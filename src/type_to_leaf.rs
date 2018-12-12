//! Output tree looks like:
//! TypedValue (List)
//!     TypeName (List)
//!         first byte (Value)
//!         ...
//!         16th byte (Value)
//!     Content (List)
//!         ...
//!
//! If Content is Value:
//! Content (List)
//!     first byte (Value)
//!     ...
//!     last byte (Value)
//!
//! Content can have 0+ bytes
//!
//! If Content is Struct/Map:
//! Content (List)
//!     Map Marker (List) (needed so we can tell empty map from empty Value)
//!         first map entry (List)
//!             ChildName (List)
//!                 first byte (Value)
//!                 ...
//!                 16th byte (Value)
//!             Children List (List)
//!                 TypedValue (List) (structure recurses here)
//!         ...
//!         last map entry (List)
//!             ...
//!
//! Content can have 0+ map entries

use super::data_models::leaf_tree::{View, Visitor};
use super::data_models::typed_value_tree::{
    ListView, ListVisitor, MapView, MapVisitor, TypeView, TypeVisitor,
};
use byteorder::WriteBytesExt;

struct ByteLister {
    v: Vec<u8>,
}

struct ByteValue(u8);

fn make_byte_lister(n: u128) -> ByteLister {
    let mut wtr = vec![];
    wtr.write_u128::<byteorder::LittleEndian>(n).unwrap();
    return ByteLister { v: wtr };
}

impl View for ByteLister {
    type Value = u8;
    fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
        for u in &self.v {
            v.visit_list(&ByteValue(*u));
        }
    }
}

impl View for ByteValue {
    type Value = u8;
    fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
        v.visit_value(self.0);
    }
}

pub struct TypeViewer<T>(pub T);
impl<T> View for TypeViewer<&T>
where
    T: TypeView<N = u128>,
{
    type Value = u8;
    fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
        let type_name = self.0.apply(TypeGetter(0u128));

        // Type: list of bytes containing type's id
        v.visit_list(&make_byte_lister(type_name.0));

        // Content: List of map entries OR List of bytes if terminal type
        v.visit_list(&ContentLister(self.0));

        struct TypeGetter(u128);
        impl TypeVisitor for TypeGetter {
            type N = u128;

            fn visit_map<T: MapView<N = Self::N>>(&mut self, type_name: &Self::N, _t: &T) {
                self.0 = *type_name;
            }
            fn visit_value(&mut self, type_name: &Self::N, _t: &[u8]) {
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

            fn visit_value(&mut self, _type_name: &Self::N, t: &[u8]) {
                // Content:
                // List of bytes for terminal type
                for u in t {
                    self.0.visit_list(&ByteValue(*u));
                }
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
                self.0.visit_list(&TypeViewer(child));
            }
        }
    }
}
