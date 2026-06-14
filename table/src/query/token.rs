use common::error::DbError;

use crate::constant::Constant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Select,
    From,
    Where,
    And,
    Insert,
    Into,
    Values,
    Delete,
    Update,
    Set,
    Create,
    Table,
    Varchar,
    Int,
    View,
    Index,
    On,
    As,
    Delimiter(char),
    Element(Constant),
}

impl Token {
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "select" => Some(Self::Select),
            "from" => Some(Self::From),
            "where" => Some(Self::Where),
            "and" => Some(Self::And),
            "insert" => Some(Self::Insert),
            "into" => Some(Self::Into),
            "values" => Some(Self::Values),
            "delete" => Some(Self::Delete),
            "update" => Some(Self::Update),
            "set" => Some(Self::Set),
            "create" => Some(Self::Create),
            "table" => Some(Self::Table),
            "varchar" => Some(Self::Varchar),
            "int" | "integer" => Some(Self::Int),
            "view" => Some(Self::View),
            "index" => Some(Self::Index),
            "on" => Some(Self::On),
            "as" => Some(Self::As),
            _ => None,
        }
    }

    pub(crate) fn is_keyword(&self) -> bool {
        !matches!(self, Self::Delimiter(_) | Self::Element(_))
    }
}

pub(crate) fn tokenize(query: &str) -> Result<Vec<Token>, DbError> {
    let mut str_char = None::<char>;
    let mut tokens = Vec::new();
    let last_idx = query.len() - 1;
    let mut token_chars = Vec::new();
    let mut prev_char = '0';

    for (i, c) in query.char_indices() {
        if is_str_token(c) || str_char.is_some() {
            if str_char == Some(c) && prev_char != '\\' {
                let token: String = token_chars.into_iter().collect();
                token_chars = Vec::new();
                tokens.push(Token::Element(Constant::Varchar(token)));
                str_char = None;
                continue;
            } else if last_idx == i && str_char.is_some() {
                return Err(DbError::EOF(format!(
                    "uexpected close tag: {}",
                    str_char.unwrap()
                )));
            } else if str_char.is_none() {
                str_char = Some(c);
            } else {
                token_chars.push(c);
            }
        } else if is_delimeter(c) || i == last_idx {
            if last_idx == i && !is_delimeter(c) {
                token_chars.push(c);
            }
            if !token_chars.is_empty() {
                let token: String = token_chars.into_iter().collect();
                if let Some(token) = Token::parse(&token.to_lowercase()) {
                    tokens.push(token);
                } else {
                    tokens.push(Token::Element(Constant::Integer(token.parse().unwrap())));
                }
            }
            if is_markable_delimeter(c) {
                tokens.push(Token::Delimiter(c));
            }
            token_chars = Vec::new();
        } else {
            token_chars.push(c);
        }
        prev_char = c;
    }
    Ok(tokens)
}

fn is_str_token(c: char) -> bool {
    c == '\'' || c == '"'
}

fn is_markable_delimeter(c: char) -> bool {
    c == '(' || c == ')' || c == ','
}

fn is_delimeter(c: char) -> bool {
    c == ' ' || c == '\n' || is_markable_delimeter(c)
}
