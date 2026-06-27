use std::{collections::HashMap, fmt::Debug, rc::Rc};

use crate::{element::Element, field_info::FieldInfo};

#[derive(Debug)]
pub struct SchemaInner {
    fields: Vec<Element>,
    infos: HashMap<Element, FieldInfo>,
}

impl SchemaInner {
    fn add_field(&mut self, fieldname: Element, field_info: FieldInfo) {
        if !self.infos.contains_key(&fieldname) {
            self.fields.push(fieldname.clone());
            self.infos.insert(fieldname, field_info);
        }
    }

    fn add_int_field(&mut self, fieldname: Element) {
        self.add_field(fieldname, FieldInfo::Integer);
    }

    fn add_string_field(&mut self, fieldname: Element, length: i32) {
        self.add_field(fieldname, FieldInfo::Varchar(length));
    }

    fn add(&mut self, fieldname: Element, schema: &Self) {
        if let Some(info) = schema.info(&fieldname) {
            self.add_field(fieldname, info);
        }
    }

    fn add_all(&mut self, schema: &Self) {
        for (field, info) in &schema.infos {
            self.add_field(field.clone(), info.clone());
        }
    }

    fn info(&self, fieldname: &Element) -> Option<FieldInfo> {
        self.infos.get(fieldname).cloned()
    }

    fn fields(&self) -> Vec<(Element, FieldInfo)> {
        let mut result = vec![];
        for field in &self.fields {
            if let Some(info) = self.infos.get(field) {
                result.push((field.clone(), info.clone()));
            }
        }
        result
    }

    fn has_field(&self, field: &Element) -> bool {
        self.infos.contains_key(field)
    }
}

#[derive(Debug, Clone)]
pub struct Schema(Rc<SchemaInner>);

impl Schema {
    pub fn info(&self, fieldname: &Element) -> Option<FieldInfo> {
        self.0.info(fieldname)
    }

    pub fn fields(&self) -> Vec<(Element, FieldInfo)> {
        self.0.fields()
    }

    pub fn has_field(&self, field: &Element) -> bool {
        self.0.has_field(field)
    }
}

pub struct SchemaBuilder {
    schema: SchemaInner,
}

impl SchemaBuilder {
    pub fn add_field(mut self, field: Element, info: FieldInfo) -> Self {
        self.schema.add_field(field, info);
        self
    }

    pub fn add_int_field(mut self, field: Element) -> Self {
        self.schema.add_int_field(field);
        self
    }

    pub fn add_string_field(mut self, field: Element, len: i32) -> Self {
        self.schema.add_string_field(field, len);
        self
    }

    pub fn add(mut self, field: Element, schema: &Schema) -> Self {
        self.schema.add(field, &schema.0);
        self
    }

    pub fn add_all(mut self, schema: &Schema) -> Self {
        self.schema.add_all(&schema.0);
        self
    }

    pub fn build(self) -> Schema {
        Schema(Rc::new(self.schema))
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self {
            schema: SchemaInner {
                fields: vec![],
                infos: HashMap::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn add() {
        let schema = SchemaBuilder::default()
            .add_int_field(Element::raw("test"))
            .build();
        let schema = SchemaBuilder::default()
            .add(Element::raw("test"), &schema)
            .build();
        assert!(schema.has_field(&Element::raw("test")));
        let schema = SchemaBuilder::default().add_all(&schema).build();
        assert!(schema.has_field(&Element::raw("test")));
        let schema = SchemaBuilder::default()
            .add_field(Element::raw("id"), FieldInfo::Integer)
            .build();
        assert!(schema.has_field(&Element::raw("id")));
    }
}
