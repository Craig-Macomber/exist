//! Each node is encoded as a type indicator (list or value), then the actual data.
//! 
//! Lists are prefixed with their count.
//! 
//! Values are written directly.
//! 
//! Other contend type id's are used for more compact optional optimizations, including:
//! - Template Tree: generates a tree (template ref + data stream)
//! - Template Sequence: generates multiple siblings (template ref + data stream)
//!
//! A template ref can either define a template inline,
//! or reference a previous template (for now by index out of all templates).
//!
//! All templates can be used for a single tree or sequence (applied multiple times, consuming sequential data).
//!
//! A template consists of a tree where some of the nodes may be replaced with generators instead of just values or lists.
//! Generators may reference the data stream and/or move the stream pointer (out of order access is allowed).
//! This gives the encoder control of the memory layout trees when desired which can enable fast path encoders and decoders for particular templates.
//!
// TODO: we could allow 0+ data streams enabling arrays of structure -> structure of arrays encoding

pub struct PrefixEncoding;
pub struct PrefixCompressedEncoding;

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
        prefix_encode(&c, &mut out);
        return out;
    }
}

impl Decoder for PrefixEncoding {
    type Value = u8;
    fn visit_root<V: Visitor<Value = Self::Value>>(&self, data: &[u8], v: &mut V) {
        let mut rdr = Cursor::new(data);
        prefix_decode(&mut rdr).visit(v);
    }
}

impl Encoder for PrefixCompressedEncoding {
    type Value = u8;
    fn serialize<TView: View<Value = Self::Value>>(&self, t: &TView) -> Vec<u8> {
        let c = view_to_concrete(t);
        let mut out = vec![];
        prefix_encode_compressed(&mut State::new(), &c, &mut out);
        return out;
    }
}

impl Decoder for PrefixCompressedEncoding {
    type Value = u8;
    fn visit_root<V: Visitor<Value = Self::Value>>(&self, data: &[u8], v: &mut V) {
        let mut rdr = Cursor::new(data);
        prefix_decode_compressed(&mut State::new(), &mut rdr).visit(v);
    }
}

const LIST_MARKER: u8 = 0;
// list with length in next 8 bytes

const VALUE_MARKER: u8 = 1;
// value is next byte

const INLINE_LIST_MIN: u8 = 128;
// 128-255 = list, inline length (subtract 128 from this byte)

// Markers used in compressed versions:

// Appends a new template
const BYTE_PATTERN_TEMPLATE_MARKER: u8 = 2;
// u32 size (length in stream)
// u8:
//      0 | 128-255 = list (length + children)
//      1 = constant value, next byte
//      2 = value from stream, followed by offset next 4 bytes
//      3 = invalid
//      4 = TEMPLATE_USE (as below, except followed by offset istead of data stream)
//      5 = TEMPLATE_USE_SEQUENCE (as below, except followed by offset istead of data stream)
//      6-127 = reserved for future use
//

// Appends a new template
// Note: the above templates populate tree values from bytes in a byte stream.
// a higher level more flexible approach (this one) would swap subtrees from a tree stream into the template to allow applying templates for trees where some subtrees are sometimes not the same shape.
const TREE_TEMPLATE_MARKER: u8 = 3;
// Template format:
// u8:
//      0 = list,
//      1 = constant value, next byte
//      2 = value from stream
//      3 = tree from stream
//      4 = TEMPLATE_USE (as below, except not followed by data stream)
//      5 = TEMPLATE_USE_SEQUENCE (as below, except not followed by data stream)
//      6-127 = reserved for future use
//      128-255 = list, inline length (subtract 128 from this byte)

const TEMPLATE_USE_MARKER: u8 = 4;
// u32: template index (TODO: varalible length encoding?)
// data stream (if BYTE_PATTERN_TEMPLATE: length = template' stride)

// Multiple nodes in a row in the same list using the same template
const TEMPLATE_USE_SEQUENCE_MARKER: u8 = 5;
// u32: template index (TODO: varalible length encoding?)
// u32: repeate count (TODO: varalible length encoding?)
// data stream (if BYTE_PATTERN_TEMPLATE: length = repeate count * template' stride)

enum Marker {
    List(usize),
    Value(u8),
    Other(u8),
}

fn read_marker<T: ReadBytesExt>(input: &mut T) -> Marker {
    let marker = input.read_u8().unwrap();
    if marker == LIST_MARKER {
        let count = input.read_u64::<LittleEndian>().unwrap();
        Marker::List(count as usize)
    } else if marker == VALUE_MARKER {
        Marker::Value(input.read_u8().unwrap())
    } else if marker >= INLINE_LIST_MIN {
        Marker::List((marker - INLINE_LIST_MIN) as usize)
    } else {
        Marker::Other(marker)
    }
}

fn write_list_marker(out: &mut Vec<u8>, length: usize) {
    if length <= (u8::max_value() - INLINE_LIST_MIN) as usize {
        out.push(length as u8 + INLINE_LIST_MIN);
    } else {
        out.push(LIST_MARKER);
        out.write_u64::<LittleEndian>(length as u64).unwrap();
    }
}

fn prefix_encode(c: &Concrete<u8>, out: &mut Vec<u8>) {
    match c {
        Concrete::List(list) => {
            write_list_marker(out, list.len());
            for child in list {
                prefix_encode(child, out);
            }
        }
        Concrete::Value(v) => {
            out.push(VALUE_MARKER);
            out.push(*v);
        }
    }
}

fn prefix_decode<T: ReadBytesExt>(input: &mut T) -> Concrete<u8> {
    let marker = read_marker(input);
    match marker {
        Marker::List(count) => {
            let mut children = vec![];
            for _i in 0..count {
                children.push(prefix_decode(input));
            }
            Concrete::List(children)
        }
        Marker::Value(value) => Concrete::Value(value),
        Marker::Other(_) => panic!(),
    }
}

struct Shape {
    counts: Vec<u32>, // the number of children in each node, in pre-order traversal order, where value nodes are encoded as u32.max_value
}

fn get_shape<TView: View<Value = u8>>(view: TView) -> Shape {
    struct Out<'a>(&'a mut Vec<u32>, usize);
    impl Visitor for Out<'_> {
        type Value = u8;
        fn visit_list<T: View<Value = u8>>(&mut self, t: &T) {
            assert!(self.0[self.1] < u32::max_value() - 1);
            self.0[self.1] = self.0[self.1] + 1;
            self.0.push(0);
            let mut out_nested = Out(self.0, self.0.len() - 1);
            t.visit(&mut out_nested);
        }
        fn visit_value(&mut self, _: u8) {
            assert_eq!(self.0[self.1], 0);
            self.0[self.1] = u32::max_value();
        }
    }

    let mut counts = vec![0];
    let mut out = Out(&mut counts, 0);
    view.visit(&mut out);

    Shape { counts }
}

struct ShapeState {
    id: u32,
    // Pushed in post order traversal order
    // TODO: store refs in here? Remove this?
    templates: Vec<Concrete<u8>>,
    // TODO: store refs in here?
    template_map: HashMap<Concrete<u8>, u32>,

    // shape + Vec<u8> = Concrete<u8>
    // when encoding check these, see if there is an existing on to reference that matches exactly (use map above)
    // can also check to find the most similar one, and make partial replace template if profitable (or maybe only do that with schema hints)
    trees: Vec<Vec<u8>>,
}

struct State {
    // Pushed in post order traversal order
    templates: Vec<Concrete<u8>>,
    template_map: HashMap<Concrete<u8>, u32>,
    //all: HashMap<Shape, ShapeState>,
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

fn prefix_encode_compressed(state: &mut State, c: &Concrete<u8>, out: &mut Vec<u8>) {
    match c {
        Concrete::List(list) => {
            let id = state.lookup(c);
            match id {
                Some(index) => {
                    out.push(TEMPLATE_USE_MARKER);
                    out.write_u32::<LittleEndian>(*index).unwrap();
                }
                None => {
                    write_list_marker(out, list.len());
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
    let marker = read_marker(input);
    match marker {
        Marker::List(count) => {
            let mut children = vec![];
            for _i in 0..count {
                children.push(prefix_decode_compressed(state, input));
            }
            let out = Concrete::List(children);
            state.record(&out);
            out
        }
        Marker::Value(value) => Concrete::Value(value),
        Marker::Other(marker) => {
            if marker == TEMPLATE_USE_MARKER {
                let index = input.read_u32::<LittleEndian>().unwrap();
                state.templates[index as usize].clone()
            } else {
                panic!()
            }
        }
    }
}
