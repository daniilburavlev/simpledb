use crate::query::data::query::ParsedQuery;
use crate::{
    element::Element,
    predicate::{Expression, Predicate},
    schema::Schema,
    value::Value,
};

pub(crate) enum Command {
    Insert(InsertData),
    Update(UpdateData),
    Query(ParsedQuery),
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

pub(crate) struct ViewData {
    pub(crate) name: String,
    pub(crate) query: ParsedQuery,
}

impl std::fmt::Display for ViewData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CREATE VIEW {} AS {}", self.name, self.query)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DeleteData {
    pub(crate) name: String,
    pub(crate) predicate: Predicate,
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

#[derive(Clone, Debug)]
pub(crate) struct UpdateData {
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
pub(crate) struct GroupByData {
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
