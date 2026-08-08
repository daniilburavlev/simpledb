use crate::{
    element::Element,
    predicate::Predicate,
    query::command::{DeleteData, GroupByData, UpdateData},
    schema_mapping::SchemaMapping,
    sort_by::SortByData,
};

pub(crate) struct ParsedQuery {
    pub(crate) fields: Vec<Element>,
    pub(crate) table: Element,
    pub(crate) predicate: Predicate,
    pub(crate) group_by: GroupByData,
    pub(crate) order_by: SortByData,
}

impl ParsedQuery {
    pub(crate) fn new(table: Element) -> Self {
        Self {
            fields: vec![],
            table,
            predicate: Predicate::default(),
            group_by: GroupByData::default(),
            order_by: SortByData::default(),
        }
    }
}

impl From<UpdateData> for ParsedQuery {
    fn from(update: UpdateData) -> Self {
        Self {
            fields: vec![update.field],
            table: Element::Raw(update.table),
            predicate: update.predicate,
            group_by: GroupByData::default(),
            order_by: SortByData::default(),
        }
    }
}

impl From<DeleteData> for ParsedQuery {
    fn from(delete: DeleteData) -> Self {
        Self {
            fields: vec![],
            table: Element::Raw(delete.name),
            predicate: delete.predicate,
            group_by: GroupByData::default(),
            order_by: SortByData::default(),
        }
    }
}

impl std::fmt::Display for ParsedQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT ")?;
        for (i, field) in self.fields.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", field)?;
            } else {
                write!(f, ", {}", field)?;
            }
        }
        write!(f, " FROM {}", self.table)?;
        let predicate = self.predicate.to_string();
        if !predicate.is_empty() {
            write!(f, " WHERE {}", predicate)?;
        }
        if !self.group_by.is_empty() {
            write!(f, " {}", self.group_by)?;
        }
        if !self.order_by.is_empty() {
            write!(f, " {}", self.order_by)?;
        }
        Ok(())
    }
}

pub(crate) struct Query {
    pub(crate) fields: Vec<Element>,
    pub(crate) tables: Vec<Element>,
    pub(crate) predicate: Predicate,
    pub(crate) group_by: GroupByData,
    pub(crate) order_by: SortByData,
    pub(crate) mapping: SchemaMapping,
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT ")?;
        for (i, field) in self.fields.iter().enumerate() {
            let field = if let Some(source) = self.mapping.field(field)
                && source != field
            {
                match source {
                    Element::Raw(source) => &Element::view(source, field.as_raw().unwrap()),
                    e => e,
                }
            } else {
                field
            };
            if i == 0 {
                write!(f, "{}", field)?;
            } else {
                write!(f, ", {}", field)?;
            }
        }
        for table in &self.tables {
            let table = if let Some(source) = self.mapping.table(table)
                && source != table
            {
                match source {
                    Element::Raw(source) => &Element::view(source, table.as_raw().unwrap()),
                    e => e,
                }
            } else {
                table
            };
            write!(f, " FROM {}", table)?;
        }
        let predicate = self.predicate.to_string();
        if !predicate.is_empty() {
            write!(f, " WHERE {}", predicate)?;
        }
        if !self.group_by.is_empty() {
            write!(f, " {}", self.group_by)?;
        }
        if !self.order_by.is_empty() {
            write!(f, " {}", self.order_by)?;
        }
        Ok(())
    }
}
