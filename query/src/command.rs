use table::{
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
                write!(f, ",{}", field)?;
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

pub struct DeleteData {
    pub name: String,
    pub predicate: Predicate,
}

pub struct InsertData {
    pub table: String,
    pub fields: Vec<String>,
    pub values: Vec<Constant>,
}

pub struct UpdateData {
    pub table: String,
    pub field: String,
    pub value: Expression,
    pub predicate: Predicate,
}

pub struct IndexData {
    pub index: String,
    pub table: String,
    pub field: String,
}

pub struct TableData {
    pub name: String,
    pub schema: Schema,
}
