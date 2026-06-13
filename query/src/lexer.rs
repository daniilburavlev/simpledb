use std::collections::{HashMap, HashSet};

use common::DbResult;

pub struct Lexer {
    keywords: HashSet<String>,
}

impl Lexer {
    pub fn new(s: &str) -> DbResult<Self> {
        let keywords = Self::init_keywords();
        Ok(Self { keywords })
    }

    fn init_keywords() -> HashSet<String> {
        let mut keywords = HashSet::new();
        keywords.insert("select".to_string());
        keywords.insert("from".to_string());
        keywords.insert("where".to_string());
        keywords.insert("and".to_string());
        keywords.insert("set".to_string());
        keywords.insert("create".to_string());
        keywords.insert("table".to_string());
        keywords.insert("varchar".to_string());
        keywords.insert("int".to_string());
        keywords.insert("integer".to_string());
        keywords.insert("view".to_string());
        keywords.insert("as".to_string());
        keywords.insert("index".to_string());
        keywords.insert("on".to_string());
        keywords
    }
}
