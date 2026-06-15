use crate::{
    constant::Constant,
    predicate::{Expression, Predicate},
    schema::Schema,
};

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
    pub fields: Vec<String>,
    pub tables: Vec<String>,
    pub predicate: Predicate,
}

impl std::fmt::Display for QueryData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT ")?;
        for (i, field) in self.fields.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", field)?;
            } else {
                write!(f, ", {}", field)?;
            }
        }
        write!(f, " FROM ")?;
        for (i, table) in self.tables.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", table)?;
            } else {
                write!(f, ", {}", table)?;
            }
        }
        let predicate = self.predicate.to_string();
        if !predicate.is_empty() {
            write!(f, " WHERE {}", predicate)?;
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
    pub fields: Vec<String>,
    pub values: Vec<Constant>,
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
    pub field: String,
    pub value: Expression,
    pub predicate: Predicate,
}

impl std::fmt::Display for UpdateData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UPDATE {} SET {}={}",
            self.table, self.field, self.value
        )?;
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
        for (i, (field, value)) in self
            .schema
            .fields()
            .map_err(|_| std::fmt::Error)?
            .iter()
            .enumerate()
        {
            if i == 0 {
                write!(f, "{} {}", field, value)?;
            } else {
                write!(f, ", {} {}", field, value)?;
            }
        }
        write!(f, ")")
    }
}
