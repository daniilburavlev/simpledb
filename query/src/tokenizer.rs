use std::sync::atomic::AtomicU16;

use common::{DbResult, error::DbError};

use crate::token::{Token, tokenize};

pub(crate) struct Tokenizer {
    current_pos: AtomicU16,
    tokens: Vec<Token>,
}

impl Tokenizer {
    pub(crate) fn new(s: &str) -> DbResult<Self> {
        let tokens = tokenize(s)?;
        Ok(Self {
            current_pos: AtomicU16::default(),
            tokens,
        })
    }

    pub(crate) fn current(&self) -> Option<Token> {
        if self.current_pos < self.tokens.len() {
            let pos = self.current_pos.load(std::sync::atomic::Ordering::SeqCst) as usize;
            return self.tokens.get(pos).cloned();
        }
        None
    }

    pub(crate) fn next(&self) -> DbResult<()> {
        let pos = self.current_pos.load(std::sync::atomic::Ordering::SeqCst) as usize;
        if pos == self.tokens.len() {
            return Err(DbError::BadSyntax);
        }
        self.current_pos
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}
