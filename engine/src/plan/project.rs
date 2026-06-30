use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use common::DbResult;

use crate::{
    element::Element,
    plan::Plan,
    scan::{Scan, project::ProjectScan},
    schema::Schema,
};
use crate::schema::SchemaBuilder;

pub struct ProjectPlan {
    plan: Rc<dyn Plan>,
    schema: Schema,
    mapping: HashMap<String, String>,
}

impl ProjectPlan {
    pub fn new(plan: Rc<dyn Plan>, fields: Vec<Element>, tables: Vec<Element>) -> DbResult<Self> {
        let schema = SchemaBuilder::default().build();
        //for field in fields {
        //    let other = plan.schema()?;
        //   schema.add(field, &other)?;
        // }
        Ok(Self {
            plan,
            schema,
            mapping: HashMap::new(),
        })
    }
}

impl Plan for ProjectPlan {
    fn open(&self) -> DbResult<Rc<dyn Scan>> {
        let scan = self.plan.open()?;
        let fields: HashSet<Element> = self.schema.fields().into_iter().map(|(f, _)| f).collect();
        Ok(Rc::new(ProjectScan::new(scan, fields)))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        self.plan.blocks_accessed()
    }

    fn records_output(&self) -> DbResult<i32> {
        self.plan.records_output()
    }

    fn distinct_values(&self, field_name: &Element) -> DbResult<i32> {
        self.plan.distinct_values(field_name)
    }

    fn schema(&self) -> DbResult<Schema> {
        Ok(self.schema.clone())
    }
}
