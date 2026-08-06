use std::rc::Rc;

use common::DbResult;

use crate::element::Element;
use crate::schema::SchemaBuilder;
use crate::{
    index_mgr::IndexInfo,
    plan::Plan,
    scan::index::{IndexJoinScan, IndexSelectScan},
    schema::Schema,
    value::Value,
};

pub struct IndexSelectPlan {
    plan: Rc<dyn Plan>,
    index: IndexInfo,
    value: Value,
}

impl IndexSelectPlan {
    pub fn new(plan: &Rc<dyn Plan>, index: IndexInfo, value: Value) -> Self {
        Self {
            plan: Rc::clone(plan),
            index,
            value,
        }
    }
}

impl Plan for IndexSelectPlan {
    fn open(&self) -> DbResult<Rc<dyn crate::scan::Scan>> {
        let ts = self.plan.open()?;
        let index = self.index.open()?;
        Ok(Rc::new(IndexSelectScan::new(
            &ts,
            &index,
            self.value.clone(),
        )?))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        Ok(self.index.block_accessed()? + self.index.records_output())
    }

    fn records_output(&self) -> DbResult<i32> {
        Ok(self.index.records_output())
    }

    fn distinct_values(&self, field_name: &Element) -> DbResult<i32> {
        Ok(self.index.distinct_values(field_name))
    }

    fn schema(&self) -> DbResult<Schema> {
        self.plan.schema()
    }
}

pub struct IndexJoinPlan {
    p1: Rc<dyn Plan>,
    p2: Rc<dyn Plan>,
    index: IndexInfo,
    field: Element,
    schema: Schema,
}

impl IndexJoinPlan {
    pub fn new(
        p1: &Rc<dyn Plan>,
        p2: &Rc<dyn Plan>,
        index: IndexInfo,
        field: Element,
    ) -> DbResult<Self> {
        let s1 = p1.schema()?;
        let s2 = p2.schema()?;
        let schema = SchemaBuilder::new(Element::Raw(format!(
            "index_join_{}_{}",
            p1.schema()?.table(),
            p2.schema()?.table()
        )))
        .add_all(&s1)
        .add_all(&s2)
        .build();
        Ok(Self {
            p1: Rc::clone(p1),
            p2: Rc::clone(p2),
            index,
            field,
            schema,
        })
    }
}

impl Plan for IndexJoinPlan {
    fn open(&self) -> DbResult<Rc<dyn crate::scan::Scan>> {
        let s = self.p1.open()?;
        let ts = self.p2.open()?;
        let idx = self.index.open()?;
        Ok(Rc::new(IndexJoinScan::new(
            &s,
            &idx,
            self.field.clone(),
            &ts,
        )?))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        Ok(self.p1.blocks_accessed()?
            + (self.p1.records_output()? * self.index.block_accessed()?)
            + self.records_output()?)
    }

    fn records_output(&self) -> DbResult<i32> {
        Ok(self.p1.records_output()? * self.index.records_output())
    }

    fn distinct_values(&self, field_name: &Element) -> DbResult<i32> {
        if self.p1.schema()?.has_field(field_name) {
            self.p1.distinct_values(field_name)
        } else {
            self.p2.distinct_values(field_name)
        }
    }

    fn schema(&self) -> DbResult<Schema> {
        Ok(self.schema.clone())
    }
}
