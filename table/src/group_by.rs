#[derive(Default)]
pub struct GroupByData {
    pub fields: Vec<String>,
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
