use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use common::DbResult;

use crate::schema::SchemaBuilder;
use crate::{element::Element, plan::Plan, scan::Scan, schema::Schema};

pub(crate) struct ProjectPlan {
    plan: Box<Plan>,
    schema: Schema,
    mapping: HashMap<String, String>,
}

impl ProjectPlan {
    pub fn new(plan: Box<Plan>, fields: Vec<Element>) -> DbResult<Self> {
        let mut schema = SchemaBuilder::default();
        for field in fields {
            let other = plan.schema()?;
            schema = schema.add(field, &other);
        }
        let schema = schema.build();
        Ok(Self {
            plan,
            schema,
            mapping: HashMap::new(),
        })
    }
}

impl ProjectPlan {
    pub(crate) fn open(&self) -> DbResult<Scan> {
        let scan = self.plan.open()?;
        let fields: HashSet<Element> = self.schema.fields().into_iter().map(|(f, _)| f).collect();
        Ok(Scan::project(Box::new(scan), fields))
    }

    pub(crate) fn blocks_accessed(&self) -> DbResult<i32> {
        self.plan.blocks_accessed()
    }

    pub(crate) fn records_output(&self) -> DbResult<i32> {
        self.plan.records_output()
    }

    pub(crate) fn distinct_values(&self, field_name: &Element) -> DbResult<i32> {
        self.plan.distinct_values(field_name)
    }

    pub(crate) fn schema(&self) -> Schema {
        self.schema.clone()
    }
}
