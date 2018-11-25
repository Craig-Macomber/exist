//! Each node is encoded as a type indicator (list or value), then the actual data.
//! Lists are prefixed with their count.
//! Values are written directly.
//! Other contend type id's are used for more compact optional optimizations, including:
//! - Template Tree: generates a tree (template ref + data stream)
//! - Template Sequence: generates multiple siblings (template ref + data stream)
//!
//! A template ref can either define a template inline,
//! or reference a previous template (for now by index out of all templates).
//!
//! All templates can be used for a single tree or sequence (applied multiple times, consuming sequential data)
//!
//! A template consists of a tree where some of the nodes may be replaced with generators instead of just values or lists.
//! Generators may reference the data stream and/or move the stream pointer (out of order access is allowed).
//! This gives the encoder control of the memory layout trees when desired which can enable fast path encoders and decoders for particular templates.
//!
// TODO: we could allow 0+ data streams enabling arrays of structure -> structure of arrays encoding

pub struct PrefixEncoding;

use super::data_models::leaf_tree::concrete::{view_to_concrete, Concrete};
use super::data_models::leaf_tree::{View, Visitor};
use super::encoding::{Decoder, Encoder};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::io::Cursor;

impl Encoder for PrefixEncoding {
    type Value = u8;
    fn serialize<TView: View<Value = Self::Value>>(&self, t: &TView) -> Vec<u8> {
        let c = view_to_concrete(t);
        let mut out = vec![];
        prefix_encode_compressed(&mut State::new(), &c, &mut out);
        return out;
    }
}

const LIST_MARKER: u8 = 0;
const VALUE_MARKER: u8 = 1;
const TEMPLATE_MARKER: u8 = 2;

struct State {
    // Pushed in post order traversal order
    templates: Vec<Concrete<u8>>,
    template_map: HashMap<Concrete<u8>, u32>,
}

impl State {
    fn new() -> State {
        State {
            templates: vec![],
            template_map: HashMap::new(),
        }
    }
    fn record(&mut self, c: &Concrete<u8>) {
        let mut inserted = false;
        let len = self.templates.len() as u32;
        self.template_map.entry(c.clone()).or_insert_with(|| {
            inserted = true;
            len
        });
        if inserted {
            self.templates.push(c.clone());
        }
    }
    fn lookup(&mut self, c: &Concrete<u8>) -> Option<&u32> {
        self.template_map.get(c)
    }
}

fn prefix_encode(state: &mut State, c: &Concrete<u8>, out: &mut Vec<u8>) {
    match c {
        Concrete::List(list) => {
            out.push(LIST_MARKER);
            out.write_u32::<LittleEndian>(list.len() as u32).unwrap();
            for child in list {
                prefix_encode(state, child, out);
            }
        }
        Concrete::Value(v) => {
            out.push(VALUE_MARKER);
            out.push(*v);
        }
    }
}

fn prefix_encode_compressed(state: &mut State, c: &Concrete<u8>, out: &mut Vec<u8>) {
    match c {
        Concrete::List(list) => {
            let id = state.lookup(c);
            match id {
                Some(index) => {
                    out.push(TEMPLATE_MARKER);
                    out.write_u32::<LittleEndian>(*index).unwrap();
                }
                None => {
                    out.push(LIST_MARKER);
                    out.write_u32::<LittleEndian>(list.len() as u32).unwrap();
                    for child in list {
                        prefix_encode_compressed(state, child, out);
                    }
                }
            }
            state.record(c);
        }
        Concrete::Value(v) => {
            out.push(VALUE_MARKER);
            out.push(*v);
        }
    }
}

fn prefix_decode_compressed<T: ReadBytesExt>(state: &mut State, input: &mut T) -> Concrete<u8> {
    let marker = input.read_u8().unwrap();
    if marker == LIST_MARKER {
        let count = input.read_u32::<LittleEndian>().unwrap();
        let mut children = vec![];
        for _i in 0..count {
            let child = prefix_decode_compressed(state, input);

            children.push(child);
        }
        let out = Concrete::List(children);
        state.record(&out);
        out
    } else if marker == TEMPLATE_MARKER {
        let index = input.read_u32::<LittleEndian>().unwrap();
        state.templates[index as usize].clone()
    } else {
        assert_eq!(marker, VALUE_MARKER);
        Concrete::Value(input.read_u8().unwrap())
    }
}

fn prefix_decode<T: ReadBytesExt>(state: &mut State, input: &mut T) -> Concrete<u8> {
    let marker = input.read_u8().unwrap();
    if marker == LIST_MARKER {
        let count = input.read_u32::<LittleEndian>().unwrap();
        let mut children = vec![];
        for _i in 0..count {
            children.push(prefix_decode(state, input));
        }
        Concrete::List(children)
    } else {
        assert_eq!(marker, VALUE_MARKER);
        Concrete::Value(input.read_u8().unwrap())
    }
}

impl Decoder for PrefixEncoding {
    type Value = u8;
    fn visit_root<V: Visitor<Value = Self::Value>>(&self, data: &[u8], v: &mut V) {
        let mut rdr = Cursor::new(data);
        prefix_decode_compressed(&mut State::new(), &mut rdr).visit(v);
    }
}
