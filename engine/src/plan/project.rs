use std::{collections::HashSet, rc::Rc, sync::Arc};

use common::DbResult;

use crate::{
    plan::Plan,
    scan::{Scan, project::ProjectScan},
    schema::Schema,
};

pub struct ProjectPlan {
    plan: Rc<dyn Plan>,
    schema: Arc<Schema>,
}

impl ProjectPlan {
    pub fn new(plan: Rc<dyn Plan>, fields: Vec<String>) -> DbResult<Self> {
        let schema = Arc::new(Schema::default());
        for field in fields {
            let other = plan.schema()?;
            schema.add(field, &other)?;
        }
        Ok(Self { plan, schema })
    }
}

impl Plan for ProjectPlan {
    fn open(&self) -> DbResult<Rc<dyn Scan>> {
        let scan = self.plan.open()?;
        let fields: HashSet<String> = self.schema.fields()?.into_iter().map(|(f, _)| f).collect();
        Ok(Rc::new(ProjectScan::new(scan, fields)))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        self.plan.blocks_accessed()
    }

    fn records_output(&self) -> DbResult<i32> {
        self.plan.records_output()
    }

    fn distinct_values(&self, field_name: &str) -> DbResult<i32> {
        self.plan.distinct_values(field_name)
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        Ok(Arc::clone(&self.schema))
    }
}
