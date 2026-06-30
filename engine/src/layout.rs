use std::{collections::HashMap, rc::Rc};

use file::page::I32_SIZE;

use crate::{element::Element, schema::Schema};

#[derive(Debug, Clone)]
struct LayoutInner {
    schema: Schema,
    offsets: HashMap<Element, i32>,
    slotsize: i32,
}

impl LayoutInner {
    fn new(schema: Schema) -> Self {
        let mut offsets = HashMap::new();
        let mut pos = I32_SIZE as i32;
        for (field, info) in schema.fields() {
            offsets.insert(field, pos);
            pos += info.length();
        }
        Self {
            schema,
            offsets,
            slotsize: pos,
        }
    }

    fn from(schema: Schema, offsets: HashMap<Element, i32>, slotsize: i32) -> Self {
        Self {
            schema,
            offsets,
            slotsize,
        }
    }

    fn offset(&self, fieldname: &Element) -> i32 {
        if let Some(offset) = self.offsets.get(fieldname) {
            *offset
        } else {
            0
        }
    }

    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn slotsize(&self) -> i32 {
        self.slotsize
    }
}

#[derive(Debug, Clone)]
pub struct Layout(Rc<LayoutInner>);

impl Layout {
    pub fn new(schema: Schema) -> Self {
        Self(Rc::new(LayoutInner::new(schema)))
    }

    pub fn from(schema: Schema, offsets: HashMap<Element, i32>, slotsize: i32) -> Self {
        Self(Rc::new(LayoutInner::from(schema, offsets, slotsize)))
    }

    pub fn offset(&self, field: &Element) -> i32 {
        self.0.offset(field)
    }

    pub fn schema(&self) -> &Schema {
        self.0.schema()
    }

    pub fn slotsize(&self) -> i32 {
        self.0.slotsize()
    }
}

#[cfg(test)]
mod tests {
    use crate::schema::SchemaBuilder;

    use super::*;

    #[test]
    fn from() {
        let schema = SchemaBuilder::default()
            .add_int_field(Element::raw("id"))
            .build();
        Layout::from(schema, HashMap::new(), 16);
    }
}
