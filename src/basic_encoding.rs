//! A simple very inefficient (space and time) encoding.

use super::data_models::leaf_tree::{View, Visitor};
use super::encoding::{Decoder, Encoder};
use byteorder::WriteBytesExt;

pub struct BasicEncoding;

const LIST_MARKER: u8 = 0;
const LIST_END: u8 = 2;
const VALUE_MARKER: u8 = 1;

impl Encoder for BasicEncoding {
    type Value = u8;
    fn serialize<TView: View<Value = Self::Value>>(&self, v: &TView) -> Vec<u8> {
        struct Output {
            vec: Vec<u8>,
        }

        impl Visitor for Output {
            type Value = u8;
            fn visit_list<T: View<Value = Self::Value>>(&mut self, t: &T) {
                self.vec.write_u8(LIST_MARKER).unwrap();
                t.visit(self);
                self.vec.write_u8(LIST_END).unwrap();
            }

            fn visit_value(&mut self, t: Self::Value) {
                self.vec.write_u8(VALUE_MARKER).unwrap();
                self.vec.write_u8(t).unwrap();
            }
        }

        return v.apply(Output { vec: vec![] }).vec;
    }
}

impl Decoder for BasicEncoding {
    type Value = u8;
    fn visit_root<V: Visitor<Value = Self::Value>>(&self, data: &[u8], v: &mut V) {
        struct Tree<'a> {
            data: &'a [u8],
        }

        impl<'a> View for Tree<'a> {
            type Value = u8;
            fn visit<V: Visitor<Value = u8>>(&self, v: &mut V) {
                if self.data.len() == 0 {
                    return;
                }
                let marker = self.data[0];
                if marker == VALUE_MARKER {
                    v.visit_value(self.data[1]);
                } else {
                    assert!(marker == LIST_MARKER);
                    let mut i = 0;
                    let mut level = 0;
                    loop {
                        if self.data.len() == i || self.data[i] == LIST_END {
                            level = level - 1;
                            if level == -1 {
                                return;
                            }
                        } else if self.data[i] == LIST_MARKER {
                            if level == 0 {
                                let sub_tree = Tree {
                                    data: &self.data[i + 1..],
                                };
                                v.visit_list(&sub_tree);
                            }
                            level = level + 1;
                        } else {
                            assert_eq!(self.data[i], VALUE_MARKER, "i = {}, level = {}", i, level);
                            if level == 0 {
                                panic!();
                            }
                            i = i + 1;
                        }
                        i = i + 1;
                    }
                }
            }
        }

        Tree { data }.visit(v);
    }
}
