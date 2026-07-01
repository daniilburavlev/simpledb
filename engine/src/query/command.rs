use crate::{
    element::Element,
    predicate::{Expression, Predicate},
    schema::Schema,
    sort_by::SortByData,
    value::Value,
};
use crate::schema_mapping::SchemaMapping;

pub enum Command {
    Insert(InsertData),
    Update(UpdateData),
    Query(QueryData),
    CreateTable(TableData),
    CreateIndex(IndexData),
    CreateView(ViewData),
    Delete(DeleteData),
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query(value) => write!(f, "{}", value),
            Self::Update(value) => write!(f, "{}", value),
            Self::Insert(value) => write!(f, "{}", value),
            Self::CreateTable(value) => write!(f, "{}", value),
            Self::CreateIndex(value) => write!(f, "{}", value),
            Self::CreateView(value) => write!(f, "{}", value),
            Self::Delete(value) => write!(f, "{}", value),
        }
    }
}

pub struct QueryData {
    pub fields: Vec<Element>,
    pub table: Element,
    pub predicate: Predicate,
    pub group_by: GroupByData,
    pub order_by: SortByData,
    pub mapping: SchemaMapping,
}

impl std::fmt::Display for QueryData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT ")?;
        for (i, field) in self.fields.iter().enumerate() {
            let field = if let Some(source) = self.mapping.field(field) && source != field {
                match source {
                    Element::Raw(source) => &Element::view(source, field.as_raw().unwrap()),
                    e => e
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
        let table = &self.table;
        let table = if let Some(source) = self.mapping.table(table) && source != table {
            match source {
                Element::Raw(source) => &Element::view(source, table.as_raw().unwrap()),
                e => e
            }
        } else {
            table
        };
        write!(f, " FROM {}", table)?;
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

pub struct ViewData {
    pub name: String,
    pub query: QueryData,
}

impl std::fmt::Display for ViewData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CREATE VIEW {} AS {}", self.name, self.query)
    }
}

pub struct DeleteData {
    pub name: String,
    pub predicate: Predicate,
}

impl std::fmt::Display for DeleteData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DELETE FROM {}", self.name)?;
        let predicate = self.predicate.to_string();
        if !predicate.is_empty() {
            write!(f, " WHERE {}", predicate)?;
        }
        Ok(())
    }
}

pub struct InsertData {
    pub table: String,
    pub fields: Vec<Element>,
    pub values: Vec<Value>,
}

impl std::fmt::Display for InsertData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "INSERT INTO {}(", self.table)?;
        for (i, field) in self.fields.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", field)?;
            } else {
                write!(f, ", {}", field)?;
            }
        }
        write!(f, ") VALUES(")?;
        for (i, value) in self.values.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", value)?;
            } else {
                write!(f, ", {}", value)?;
            }
        }
        write!(f, ")")?;
        Ok(())
    }
}

pub struct UpdateData {
    pub table: String,
    pub field: Element,
    pub value: Expression,
    pub predicate: Predicate,
}

impl std::fmt::Display for UpdateData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UPDATE {} SET {}={}", self.table, self.field, self.value)?;
        let predicate = self.predicate.to_string();
        if !predicate.is_empty() {
            write!(f, " WHERE {}", predicate)?;
        }
        Ok(())
    }
}

pub struct IndexData {
    pub index: String,
    pub table: String,
    pub field: String,
}

impl std::fmt::Display for IndexData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CREATE INDEX {} ON {}({})",
            self.index, self.table, self.field
        )
    }
}

pub struct TableData {
    pub name: String,
    pub schema: Schema,
}

impl std::fmt::Display for TableData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CREATE TABLE {}(", self.name)?;
        for (i, (field, value)) in self.schema.fields().iter().enumerate() {
            if i == 0 {
                write!(f, "{} {}", field, value)?;
            } else {
                write!(f, ", {} {}", field, value)?;
            }
        }
        write!(f, ")")
    }
}

#[derive(Default)]
pub struct GroupByData {
    pub fields: Vec<Element>,
}

impl GroupByData {
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

impl std::fmt::Display for GroupByData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GROUP BY ")?;
        for (i, field) in self.fields.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", field)?;
            } else {
                write!(f, ",{}", field)?;
            }
        }
        Ok(())
    }
}
