use crate::{element::Element, value::Value};

pub(crate) struct ParsedInsertQuery {
    pub(crate) fields: Vec<Element>,
    pub(crate) table: Element,
    pub(crate) values: Vec<Vec<Value>>,
}

impl std::fmt::Display for ParsedInsertQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "INSERT INTO {}(", self.table)?;
        for (i, field) in self.fields.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", field)?;
            } else {
                write!(f, ", {}", field)?;
            }
        }
        write!(f, ") VALUES")?;
        let len = self.values.len();
        for (i, values) in self.values.iter().enumerate() {
            write!(f, "(")?;
            for (j, value) in values.iter().enumerate() {
                if j == 0 {
                    write!(f, "{}", value)?;
                } else {
                    write!(f, ", {}", value)?;
                }
            }
            if len > 1 && i < len - 1 {
                write!(f, "), ")?;
            } else {
                write!(f, ")")?;
            }
        }
        Ok(())
    }
}

pub(crate) struct InsertQuery {
    pub(crate) fields: Vec<Element>,
    pub(crate) table: Element,
    pub(crate) values: Vec<Vec<Value>>,
}
