use common::error::DbError;
use planner::value::Value;

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
    Join,
    On,
    As,
    Group,
    Sort,
    By,
    Field(String),
    Delimiter(char),
    Element(Value),
}

impl Token {
    fn parse(token: &str) -> Option<Self> {
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
            "join" => Some(Self::Join),
            "on" => Some(Self::On),
            "as" => Some(Self::As),
            "group" => Some(Self::Group),
            "sort" => Some(Self::Sort),
            "by" => Some(Self::By),
            _ => None,
        }
    }

    pub(crate) fn is_keyword(&self) -> bool {
        !matches!(self, Self::Field(_) | Self::Delimiter(_) | Self::Element(_))
    }
}

pub(crate) fn tokenize(query: &str) -> Result<Vec<Token>, DbError> {
    let mut str_char = None::<char>;
    let mut tokens = Vec::new();
    let last_idx = if query.is_empty() { 0 } else { query.len() - 1 };
    let mut token_chars = Vec::new();
    let mut prev_char = '0';

    for (i, c) in query.char_indices() {
        if is_str_token(c) || str_char.is_some() {
            if str_char == Some(c) && prev_char != '\\' {
                let token: String = token_chars.into_iter().collect();
                token_chars = Vec::new();
                tokens.push(Token::Element(Value::Varchar(token)));
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
                } else if let Ok(value) = token.parse::<i32>() {
                    tokens.push(Token::Element(Value::Integer(value)));
                } else {
                    tokens.push(Token::Field(token))
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
    c == '(' || c == ')' || c == ',' || c == '=' || c == '.'
}

fn is_delimeter(c: char) -> bool {
    c == ' ' || c == '\n' || is_markable_delimeter(c)
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Select => write!(f, "SELECT"),
            Token::From => write!(f, "FROM"),
            Token::Where => write!(f, "WHERE"),
            Token::And => write!(f, "AND"),
            Token::Insert => write!(f, "INSERT"),
            Token::Into => write!(f, "INTO"),
            Token::Values => write!(f, "VALUES"),
            Token::Delete => write!(f, "DELETE"),
            Token::Update => write!(f, "UPDATE"),
            Token::Set => write!(f, "SET"),
            Token::Create => write!(f, "CREATE"),
            Token::Table => write!(f, "TABLE"),
            Token::Varchar => write!(f, "VARCHAR"),
            Token::Int => write!(f, "INT"),
            Token::View => write!(f, "VIEW"),
            Token::Index => write!(f, "INDEX"),
            Token::Join => write!(f, "JOIN"),
            Token::On => write!(f, "ON"),
            Token::As => write!(f, "AS"),
            Token::Group => write!(f, "GROUP"),
            Token::Sort => write!(f, "SORT"),
            Token::By => write!(f, "BY"),
            Token::Field(field) => write!(f, "'{}'", field),
            Token::Delimiter(d) => write!(f, "'{}'", d),
            Token::Element(constant) => write!(f, "'{}'", constant),
        }
    }
}
