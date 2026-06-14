use std::{
    rc::Rc,
    sync::{Arc, RwLock},
};

use common::{DbResult, error::DbError};

use crate::{constant::Constant, plan::Plan, scan::Scan, schema::Schema};

#[derive(Clone)]
pub enum Expression {
    Value(Constant),
    Field(String),
}

impl Expression {
    pub fn evaluate(&self, scan: &Rc<dyn Scan>) -> DbResult<Constant> {
        match self {
            Self::Value(value) => Ok(value.clone()),
            Self::Field(field) => scan.get_val(field),
        }
    }

    pub fn applies_to(&self, schema: &Schema) -> DbResult<bool> {
        match self {
            Self::Value(_) => Ok(true),
            Self::Field(field) => schema.has_field(field),
        }
    }

    pub fn as_field_name(&self) -> Option<&str> {
        match self {
            Self::Field(field) => Some(field),
            _ => None,
        }
    }

    pub fn as_constant(&self) -> Option<&Constant> {
        match self {
            Self::Value(value) => Some(value),
            _ => None,
        }
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(value) => write!(f, "{}", value),
            Self::Field(field) => write!(f, "{}", field),
        }
    }
}

#[derive(Clone)]
pub struct Term {
    left: Expression,
    right: Expression,
}

impl Term {
    pub fn new(left: Expression, right: Expression) -> Self {
        Self { left, right }
    }

    pub fn is_satisfied(&self, s: &Rc<dyn Scan>) -> DbResult<bool> {
        let left = self.left.evaluate(s)?;
        let right = self.right.evaluate(s)?;
        Ok(left == right)
    }

    pub fn applies_to(&self, schema: &Schema) -> DbResult<bool> {
        Ok(self.left.applies_to(schema)? && self.right.applies_to(schema)?)
    }

    pub fn reduction_factor(&self, p: &Arc<dyn Plan>) -> DbResult<i32> {
        if let Some(left) = self.left.as_field_name()
            && let Some(right) = self.right.as_field_name()
        {
            return Ok(p.distinct_values(left)?.max(p.distinct_values(right)?));
        }
        if let Some(left) = self.left.as_field_name() {
            return p.distinct_values(left);
        }
        if let Some(right) = self.right.as_field_name() {
            return p.distinct_values(right);
        }
        if let Some(left) = self.left.as_constant()
            && let Some(right) = self.right.as_constant()
            && left == right
        {
            Ok(1)
        } else {
            Ok(i32::MAX)
        }
    }

    pub fn equates_with_constant(&self, field_name: &str) -> DbResult<Option<Constant>> {
        if let Some(field) = self.left.as_field_name()
            && field == field_name
            && let Some(value) = self.right.as_constant()
        {
            Ok(Some(value.clone()))
        } else if let Some(right) = self.right.as_field_name()
            && right == field_name
            && let Some(value) = self.left.as_constant()
        {
            Ok(Some(value.clone()))
        } else {
            Ok(None)
        }
    }

    pub fn equates_with_field(&self, field_name: &str) -> DbResult<Option<String>> {
        if let Some(field) = self.left.as_field_name()
            && field == field_name
            && let None = self.right.as_field_name()
        {
            Ok(Some(field.to_string()))
        } else if let Some(field) = self.right.as_field_name()
            && field == field_name
            && let Some(_) = self.left.as_field_name()
        {
            Ok(Some(field.to_string()))
        } else {
            Ok(None)
        }
    }
}

impl std::fmt::Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.left, self.right)
    }
}

#[derive(Default, Clone)]
pub struct Predicate {
    terms: Arc<RwLock<Vec<Term>>>,
}

impl Predicate {
    pub fn new(term: Term) -> Self {
        Self {
            terms: Arc::new(RwLock::new(vec![term])),
        }
    }

    pub fn conjoin_with(&self, p: &Self) -> DbResult<()> {
        let mut write = self.terms.write().map_err(DbError::lock)?;
        let read = p.terms.read().map_err(DbError::lock)?;
        for term in read.iter() {
            write.push(term.clone());
        }
        Ok(())
    }

    pub fn is_satisfied(&self, s: &Rc<dyn Scan>) -> DbResult<bool> {
        let read = self.terms.read().map_err(DbError::lock)?;
        for t in read.iter() {
            if !t.is_satisfied(s)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn reduction_factor(&self, p: &Arc<dyn Plan>) -> DbResult<i32> {
        let mut factor = 1;
        let read = self.terms.read().map_err(DbError::lock)?;
        for t in read.iter() {
            factor *= t.reduction_factor(p)?;
        }
        Ok(factor)
    }

    pub fn select_sub_pred(&self, schema: &Schema) -> DbResult<Predicate> {
        let result = Predicate::default();
        let read = self.terms.read().map_err(DbError::lock)?;
        {
            let mut terms = result.terms.write().map_err(DbError::lock)?;
            for t in read.iter() {
                if t.applies_to(schema)? {
                    terms.push(t.clone());
                }
            }
        }
        Ok(result)
    }

    pub fn join_sub_pred(&self, s1: &Schema, s2: &Schema) -> DbResult<Option<Predicate>> {
        let result = Predicate::default();
        let new_schema = Schema::default();
        new_schema.add_all(s1)?;
        new_schema.add_all(s2)?;
        let read = self.terms.read().map_err(DbError::lock)?;
        {
            let mut terms = result.terms.write().map_err(DbError::lock)?;
            for t in read.iter() {
                if !t.applies_to(s1)? && !t.applies_to(s2)? && t.applies_to(&new_schema)? {
                    terms.push(t.clone());
                }
            }
            if terms.is_empty() {
                return Ok(None);
            }
        }
        Ok(Some(result))
    }

    pub fn equates_with_constant(&self, field_name: &str) -> DbResult<Option<Constant>> {
        let terms = self.terms.read().map_err(DbError::lock)?;
        for t in terms.iter() {
            if let Some(c) = t.equates_with_constant(field_name)? {
                return Ok(Some(c));
            }
        }
        Ok(None)
    }

    pub fn equates_with_field(&self, field_name: &str) -> DbResult<Option<String>> {
        let terms = self.terms.read().map_err(DbError::lock)?;
        for t in terms.iter() {
            if let Some(field) = t.equates_with_field(field_name)? {
                return Ok(Some(field));
            }
        }
        Ok(None)
    }
}

impl std::fmt::Display for Predicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let terms = self.terms.read().map_err(|_| std::fmt::Error)?;
        for (i, t) in terms.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", t)?;
            } else {
                write!(f, " and {}", t)?;
            }
        }
        Ok(())
    }
}
