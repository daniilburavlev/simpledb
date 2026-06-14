use std::{collections::HashSet, sync::Arc};

use common::DbResult;

use crate::{
    plan::Plan,
    scan::{Scan, project::ProjectScan},
    schema::Schema,
};

pub struct ProjectPlan {
    plan: Box<dyn Plan>,
    schema: Arc<Schema>,
}

impl ProjectPlan {
    pub fn new(plan: Box<dyn Plan>) -> Self {
        Self {
            plan,
            schema: Arc::new(Schema::default()),
        }
    }

    pub fn open(&self) -> DbResult<Box<dyn Scan>> {
        let scan = self.plan.open()?;
        let fields: HashSet<String> = self.schema.fields()?.into_iter().map(|(f, _)| f).collect();
        Ok(Box::new(ProjectScan::new(scan, fields)))
    }

    pub fn blocks_accessed(&self) -> DbResult<i32> {
        self.plan.blocks_accessed()
    }

    pub fn records_output(&self) -> DbResult<i32> {
        self.plan.records_output()
    }

    pub fn distinct_values(&self, field_name: &str) -> DbResult<i32> {
        self.plan.distinct_values(field_name)
    }

    pub fn schema(&self) -> DbResult<Arc<Schema>> {
        Ok(Arc::clone(&self.schema))
    }
}
