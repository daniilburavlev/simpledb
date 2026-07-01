use crate::element::Element;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq, Default)]
struct SchemaMappingInner {
    tables_fields: HashMap<Element, HashSet<Element>>,
    tables_names: HashMap<Element, Element>,
    fields_names: HashMap<Element, Element>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SchemaMapping(Rc<SchemaMappingInner>);

impl SchemaMapping {
    pub(crate) fn field(&self, name: &Element) -> Option<&Element> {
        self.0.fields_names.get(name)
    }

    pub(crate) fn table(&self, name: &Element) -> Option<&Element> {
        self.0.tables_names.get(name)
    }
}

#[derive(Default)]
pub(crate) struct SchemaMappingBuilder(SchemaMappingInner);

impl SchemaMappingBuilder {
    pub(crate) fn add_table_field(mut self, table: Element, field: Element) -> Self {
        let fields = self.0.tables_fields.entry(table).or_insert(HashSet::new());
        fields.insert(field);
        self
    }

    pub(crate) fn add_field(mut self, table: Element, id: Element, source: Element) -> Self {
        self.0.fields_names.insert(id, source.clone());
        let fields = self
            .0
            .tables_fields
            .entry(table)
            .or_insert_with(HashSet::new);
        fields.insert(source);
        self
    }

    pub(crate) fn add_table(mut self, id: Element, source: Element) -> Self {
        self.0.tables_names.insert(id, source);
        self
    }

    pub(crate) fn build(self) -> SchemaMapping {
        SchemaMapping(Rc::new(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_mapping() {
        let source_table = Element::raw("table");
        let id_table = Element::raw("t");

        let source_field = Element::raw("id");
        let id_field = Element::raw("i");

        let mut fields = HashSet::new();
        fields.insert(source_field.clone());

        let mapping = SchemaMappingBuilder::default()
            .add_table(id_table.clone(), source_table.clone())
            .add_field(source_table.clone(), id_field.clone(), source_field.clone())
            .build();

        assert_eq!(source_table, *mapping.table(&id_table).unwrap());
        assert_eq!(source_field, *mapping.field(&id_field).unwrap());
    }
}
