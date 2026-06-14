use std::sync::Arc;

use common::DbResult;

use crate::{
    plan::Plan,
    scan::{Scan, product::ProductScan},
    schema::Schema,
};

pub struct ProductPlan {
    p1: Box<dyn Plan>,
    p2: Box<dyn Plan>,
    schema: Arc<Schema>,
}

impl ProductPlan {
    pub fn new(p1: Box<dyn Plan>, p2: Box<dyn Plan>) -> DbResult<Self> {
        let schema = Arc::new(Schema::default());
        let s1 = p1.schema()?;
        let s2 = p2.schema()?;
        schema.add_all(&s1)?;
        schema.add_all(&s2)?;
        Ok(Self { p1, p2, schema })
    }
}

impl Plan for ProductPlan {
    fn open(&self) -> DbResult<Box<dyn Scan>> {
        let s1 = self.p1.open()?;
        let s2 = self.p2.open()?;
        Ok(Box::new(ProductScan::new(s1, s2)?))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        Ok(self.p1.blocks_accessed()? + (self.p1.records_output()? * self.p2.blocks_accessed()?))
    }

    fn records_output(&self) -> DbResult<i32> {
        Ok(self.p1.records_output()? * self.p2.records_output()?)
    }

    fn distinct_values(&self, field_name: &str) -> DbResult<i32> {
        if self.p1.schema()?.has_field(field_name)? {
            self.p1.distinct_values(field_name)
        } else {
            self.p2.distinct_values(field_name)
        }
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        Ok(Arc::clone(&self.schema))
    }
}
