pub struct BytePatternTemplate<'a> {
    size: u32,
    content: BytePatternChild<'a>,
}

pub enum BytePatternChild<'a> {
    List(Vec<BytePatternChild<'a>>),
    ConstantValue(u8),
    ValueFromStreamAtOffset(u32),
    TemplateUse(OffsetTemplateUse<&'a BytePatternTemplate<'a>>),
}

pub struct OffsetTemplateUse<T> {
    template: T,
    offset: u32,
}

pub enum TreeTemplate<'a> {
    List(Vec<TreeTemplate<'a>>),
    ConstantValue(u8),
    ValueFromStream,
    TreeFromStream,
    TreeTemplateUse(&'a TreeTemplate<'a>),
    BytePatternTemplateUse(&'a BytePatternTemplate<'a>),
}

use super::data_models::leaf_tree::concrete::{view_to_concrete, Concrete};
use super::data_models::leaf_tree::View;

pub fn assert_compliance<TView: View<Value = u8>>(view: &TView, template: &TreeTemplate) {
    let c = view_to_concrete(view);
    assert_compliance_concreate(&c, template);
}

fn assert_compliance_concreate(c: &Concrete<u8>, template: &TreeTemplate) {
    match template {
        TreeTemplate::List(children) => {
            match c {
                Concrete::List(list) => {
                    assert_eq!(list.len(), children.len());
                    for i in 0..list.len() {
                        assert_compliance_concreate(&list[i], &children[i]);
                    }
                }
                Concrete::Value(_) => panic!(),
            };
        }
        TreeTemplate::ConstantValue(value) => {
            match c {
                Concrete::List(_) => panic!(),
                Concrete::Value(tree_value) => assert_eq!(value, tree_value),
            };
        }
        TreeTemplate::ValueFromStream => {
            match c {
                Concrete::List(_) => panic!(),
                Concrete::Value(_) => {}
            };
        }
        TreeTemplate::TreeFromStream => {}
        TreeTemplate::TreeTemplateUse(template) => assert_compliance_concreate(c, template),
        TreeTemplate::BytePatternTemplateUse(_) => panic!("Not Implemented"),
    }
}

/// Assumes template does not contain overlaps (multiple places in the tree sourced from the same location in the template)
pub fn assert_pattern_compliance<TView: View<Value = u8>>(
    view: &TView,
    template: &BytePatternTemplate,
) {
    let c = view_to_concrete(view);
    assert_pattern_compliance_concreate(&c, &template.content);
}

fn assert_pattern_compliance_concreate(c: &Concrete<u8>, template: &BytePatternChild) {
    match template {
        BytePatternChild::List(children) => {
            match c {
                Concrete::List(list) => {
                    assert_eq!(list.len(), children.len());
                    for i in 0..list.len() {
                        assert_pattern_compliance_concreate(&list[i], &children[i]);
                    }
                }
                Concrete::Value(_) => panic!(),
            };
        }
        BytePatternChild::ConstantValue(value) => {
            match c {
                Concrete::List(_) => panic!(),
                Concrete::Value(tree_value) => assert_eq!(value, tree_value),
            };
        }
        BytePatternChild::ValueFromStreamAtOffset(_) => {
            match c {
                Concrete::List(_) => panic!(),
                Concrete::Value(_) => {}
            };
        }
        BytePatternChild::TemplateUse(template) => {
            assert_pattern_compliance_concreate(c, &template.template.content)
        }
    }
}
