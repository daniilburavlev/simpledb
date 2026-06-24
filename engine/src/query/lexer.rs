use common::{DbResult, error::DbError};

use crate::{
    constant::Constant,
    query::{token::Token, tokenizer::Tokenizer},
};

pub struct Lexer {
    tokenizer: Tokenizer,
}

impl Lexer {
    pub fn new(s: &str) -> DbResult<Self> {
        Ok(Self {
            tokenizer: Tokenizer::new(s)?,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.tokenizer.is_empty()
    }

    pub fn match_delim(&self, c: char) -> bool {
        self.tokenizer.current() == Some(Token::Delimiter(c))
    }

    pub fn match_int_constant(&self) -> bool {
        matches!(
            self.tokenizer.current(),
            Some(Token::Element(Constant::Integer(_)))
        )
    }

    pub fn match_string_constant(&self) -> bool {
        matches!(
            self.tokenizer.current(),
            Some(Token::Element(Constant::Varchar(_)))
        )
    }

    pub fn match_keyword(&self, expected: Token) -> bool {
        if let Some(token) = self.tokenizer.current()
            && token == expected
        {
            return true;
        }
        false
    }

    pub fn match_id(&self) -> bool {
        if let Some(token) = self.tokenizer.current()
            && matches!(token, Token::Field(_))
            && !token.is_keyword()
        {
            return true;
        }
        false
    }

    pub fn eat_delimiter(&self, c: char) -> DbResult<()> {
        if !self.match_delim(c) {
            return Err(DbError::BadSyntax);
        }
        self.tokenizer.next()?;
        Ok(())
    }

    pub fn eat_int_constant(&self) -> DbResult<Constant> {
        if let Some(Token::Element(Constant::Integer(value))) = self.tokenizer.current() {
            self.tokenizer.next()?;
            Ok(Constant::Integer(value))
        } else {
            Err(DbError::BadSyntax)
        }
    }

    pub fn eat_string_constant(&self) -> DbResult<Constant> {
        if let Some(Token::Element(Constant::Varchar(value))) = self.tokenizer.current() {
            self.tokenizer.next()?;
            Ok(Constant::Varchar(value))
        } else {
            Err(DbError::BadSyntax)
        }
    }

    pub fn eat_keyword(&self, expected: Token) -> DbResult<()> {
        if let Some(token) = self.tokenizer.current()
            && token == expected
        {
            self.tokenizer.next()?;
            return Ok(());
        }
        Err(DbError::BadSyntax)
    }

    pub fn eat_id(&self) -> DbResult<String> {
        if let Some(Token::Field(id)) = self.tokenizer.current() {
            self.tokenizer.next()?;
            return Ok(id);
        }
        Err(DbError::BadSyntax)
    }

    pub fn eat(&self) -> DbResult<Token> {
        if let Some(token) = self.tokenizer.current() {
            self.tokenizer.next()?;
            return Ok(token);
        }
        Err(DbError::BadSyntax)
    }
}
