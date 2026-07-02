use std::{
    fmt::Debug,
    rc::Rc,
    sync::{Arc, RwLock},
};

use common::{DbResult, error::DbError};

use crate::element::Element;
use crate::schema::SchemaBuilder;
use crate::schema_mapping::SchemaMapping;
use crate::{plan::Plan, scan::Scan, schema::Schema, value::Value};

#[derive(Debug, Clone)]
pub enum Expression {
    Value(Value),
    Field(Element),
}

impl Expression {
    pub fn evaluate(&self, scan: &Rc<dyn Scan>) -> DbResult<Value> {
        match self {
            Self::Value(value) => Ok(value.clone()),
            Self::Field(field) => scan.get_val(field),
        }
    }

    pub fn applies_to(&self, schema: &Schema) -> bool {
        match self {
            Self::Value(_) => true,
            Self::Field(field) => schema.has_field(field),
        }
    }

    pub(crate) fn applies_to_table(&self, schema: &Schema, mapping: &SchemaMapping) -> bool {
        let Self::Field(field) = self else {
            return true;
        };
        match field {
            Element::Raw(field) => {
                let field = if let Some(field) = mapping.field(&Element::raw(field)) {
                    field
                } else {
                    &Element::raw(field)
                };
                schema.has_field(field)
            }
            Element::Spec(source, target) => {
                let table = if let Some(table) = mapping.table(&Element::raw(source)) {
                    table
                } else {
                    &Element::raw(source)
                };
                if schema.table() != table {
                    return false;
                }
                let field = Element::raw(target);
                schema.has_field(&field)
            }
            _ => false,
        }
    }

    pub fn as_field_name(&self) -> Option<&Element> {
        match self {
            Self::Field(field) => Some(field),
            _ => None,
        }
    }

    pub fn as_constant(&self) -> Option<&Value> {
        match self {
            Self::Value(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn resolve(&self, mapping: &SchemaMapping) -> Self {
        let Self::Field(field) = self else {
            return self.clone();
        };
        let field = match field {
            Element::Raw(field) => {
                if let Some(field) = mapping.field(&Element::raw(field)) {
                    field.clone()
                } else {
                    Element::raw(field)
                }
            }
            Element::Spec(_, target) => Element::raw(target),
            _ => panic!("unprocessable field"),
        };
        Self::Field(field)
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

#[derive(Clone, Debug)]
pub(crate) struct Term {
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

    pub fn applies_to(&self, schema: &Schema) -> bool {
        self.left.applies_to(schema) && self.right.applies_to(schema)
    }

    pub(crate) fn applies_to_table(&self, schema: &Schema, mapping: &SchemaMapping) -> bool {
        self.left.applies_to_table(schema, mapping) && self.right.applies_to_table(schema, mapping)
    }

    pub fn reduction_factor(&self, p: &Rc<dyn Plan>) -> DbResult<i32> {
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

    pub fn equates_with_constant(&self, field_name: &Element) -> DbResult<Option<Value>> {
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

    pub fn equates_with_field(&self, field_name: &Element) -> DbResult<Option<Element>> {
        if let Some(left) = self.left.as_field_name()
            && left == field_name
            && let Some(right) = self.right.as_field_name()
        {
            Ok(Some(right.clone()))
        } else if let Some(right) = self.right.as_field_name()
            && right == field_name
            && let Some(left) = self.left.as_field_name()
        {
            Ok(Some(left.clone()))
        } else {
            Ok(None)
        }
    }

    pub fn resolve(&self, mapping: &SchemaMapping) -> Term {
        let left = self.left.resolve(mapping);
        let right = self.right.resolve(mapping);
        Term::new(left, right)
    }
}

impl std::fmt::Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.left, self.right)
    }
}

#[derive(Clone)]
pub(crate) struct Predicate {
    terms: Arc<RwLock<Vec<Term>>>,
}

impl Default for Predicate {
    fn default() -> Self {
        Self {
            terms: Arc::new(RwLock::new(vec![])),
        }
    }
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

    pub fn reduction_factor(&self, p: &Rc<dyn Plan>) -> DbResult<i32> {
        let mut factor = 1;
        let read = self.terms.read().map_err(DbError::lock)?;
        for t in read.iter() {
            factor *= t.reduction_factor(p)?;
        }
        Ok(factor)
    }

    pub(crate) fn select_sub_pred(
        &self,
        schema: &Schema,
        mapping: &SchemaMapping,
    ) -> DbResult<Predicate> {
        let result = Predicate::default();
        let read = self.terms.read().map_err(DbError::lock)?;
        {
            let mut terms = result.terms.write().map_err(DbError::lock)?;
            for t in read.iter() {
                if t.applies_to_table(schema, mapping) {
                    let term = t.resolve(mapping);
                    terms.push(term);
                }
            }
        }
        Ok(result)
    }

    pub(crate) fn join_sub_pred(&self, s1: &Schema, s2: &Schema) -> DbResult<Option<Predicate>> {
        let result = Predicate::default();
        let new_schema =
            SchemaBuilder::new(Element::Raw(format!("join_{}_{}", s1.table(), s2.table())))
                .add_all(s1)
                .add_all(s2)
                .build();
        let read = self.terms.read().map_err(DbError::lock)?;
        {
            let mut terms = result.terms.write().map_err(DbError::lock)?;
            for t in read.iter() {
                if !t.applies_to(s1) && !t.applies_to(s2) && t.applies_to(&new_schema) {
                    terms.push(t.clone());
                }
            }
            if terms.is_empty() {
                return Ok(None);
            }
        }
        Ok(Some(result))
    }

    pub fn equates_with_constant(&self, field_name: &Element) -> DbResult<Option<Value>> {
        let terms = self.terms.read().map_err(DbError::lock)?;
        for t in terms.iter() {
            if let Some(c) = t.equates_with_constant(field_name)? {
                return Ok(Some(c));
            }
        }
        Ok(None)
    }

    pub fn equates_with_field(&self, field_name: &Element) -> DbResult<Option<Element>> {
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
                write!(f, " AND {}", t)?;
            }
        }
        Ok(())
    }
}
