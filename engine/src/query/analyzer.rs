use std::{collections::HashMap, sync::Arc};

use common::{DbResult, error::DbError};
use transaction::transaction::Transaction;

use crate::{
    element::Element,
    layout::Layout,
    metadata_mgr::MetadataMgr,
    query::command::{
        insert::{InsertQuery, ParsedInsertQuery},
        select::{ParsedSelectQuery, SelectQuery},
    },
    schema_mapping::SchemaMappingBuilder,
};

pub(crate) struct Analyzer {
    md: MetadataMgr,
    tx: Arc<Transaction>,
}

impl Analyzer {
    pub(crate) fn new(md: MetadataMgr, tx: &Arc<Transaction>) -> Self {
        Self {
            md,
            tx: Arc::clone(tx),
        }
    }

    pub(crate) fn query(&self, data: ParsedSelectQuery) -> DbResult<SelectQuery> {
        let raw_tables = match data.table {
            Element::Array(tables) => tables,
            table => vec![table],
        };
        let mut tables: Vec<Element> = Vec::with_capacity(raw_tables.len());
        let mut tables_layouts: HashMap<usize, Layout> = HashMap::with_capacity(raw_tables.len());
        let mut mapping = SchemaMappingBuilder::default();
        for (i, table) in raw_tables.into_iter().enumerate() {
            let (table, layout) = match table {
                Element::Raw(table) => {
                    let layout = self.md.get_layout(&table, &self.tx)?;
                    (Element::Raw(table), layout)
                }
                Element::View(source, id) => {
                    let layout = self.md.get_layout(&source, &self.tx)?;
                    mapping = mapping.add_table(Element::raw(&id), Element::raw(&source));
                    (Element::Raw(id), layout)
                }
                _ => return Err(DbError::InvalidFieldType),
            };
            tables.push(table);
            tables_layouts.insert(i, layout);
        }
        let mut fields = Vec::with_capacity(data.fields.len());
        for field in data.fields {
            let mut existed = false;
            for (i, table) in tables.iter().enumerate() {
                let Some(layout) = tables_layouts.get(&i) else {
                    return Err(DbError::BadSyntax);
                };
                match &field {
                    Element::Raw(name) => {
                        let field = Element::raw(name);
                        if !layout.schema().has_field(&field) {
                            continue;
                        }
                        if existed {
                            return Err(DbError::specify(name));
                        }
                        existed = true;
                        mapping = mapping.add_table_field(table.clone(), field.clone());
                        fields.push(field);
                    }
                    Element::View(source_name, id) => {
                        let source = Element::raw(source_name);
                        if !layout.schema().has_field(&source) {
                            continue;
                        }
                        if existed {
                            return Err(DbError::specify(source_name));
                        }
                        existed = true;
                        mapping = mapping.add_table_field(table.clone(), source.clone());
                        mapping = mapping.add_field(table.clone(), Element::raw(id), source);
                        fields.push(Element::raw(id))
                    }
                    Element::Spec(table_name, field_name) => {
                        let curr_table = Element::raw(table_name);
                        if *table != curr_table {
                            continue;
                        }
                        let field = Element::raw(field_name);
                        if !layout.schema().has_field(&field) {
                            continue;
                        }
                        if existed {
                            return Err(DbError::specify(field_name));
                        }
                        existed = true;
                        mapping = mapping.add_table_field(curr_table, field);
                        fields.push(Element::spec(table_name, field_name))
                    }
                    _ => return Err(DbError::InvalidFieldType),
                }
            }
            if !existed {
                return Err(DbError::FieldNotExists(field.to_string()));
            }
        }
        let mapping = mapping.build();
        Ok(SelectQuery {
            fields,
            tables,
            predicate: data.predicate,
            group_by: data.group_by,
            order_by: data.order_by,
            mapping,
        })
    }

    pub(crate) fn insert(&self, data: ParsedInsertQuery) -> DbResult<InsertQuery> {
        let table = data.table;
        if !matches!(table, Element::Raw(_)) {
            return Err(DbError::InvalidFieldType);
        }
        let fields = data.fields;
        let values = data.values;
        for values in &values {
            if fields.len() != values.len() {
                return Err(DbError::InvalidValuesAmount);
            }
        }
        let layout = self.md.get_layout(table.as_raw()?, &self.tx)?;
        for field in &fields {
            if !matches!(field, Element::Raw(_)) {
                return Err(DbError::InvalidFieldType);
            }
            if !layout.schema().has_field(field) {
                return Err(DbError::field_not_exists(field.as_raw()?));
            }
        }
        Ok(InsertQuery {
            fields,
            table,
            values,
        })
    }
}
