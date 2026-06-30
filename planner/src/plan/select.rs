use common::DbResult;

use crate::{element::Element, plan::Plan, predicate::Predicate, scan::Scan, schema::Schema};

pub(crate) struct SelectPlan {
    plan: Box<Plan>,
    predicate: Predicate,
}

impl SelectPlan {
    pub(crate) fn new(plan: Box<Plan>, predicate: Predicate) -> Self {
        Self { plan, predicate }
    }

    pub(crate) fn open(&self) -> DbResult<Scan> {
        let s = self.plan.open()?;
        Ok(Scan::select(Box::new(s), self.predicate.clone()))
    }

    pub(crate) fn blocks_accessed(&self) -> DbResult<i32> {
        self.plan.blocks_accessed()
    }

    pub(crate) fn records_output(&self) -> DbResult<i32> {
        Ok(self.plan.records_output()? / self.predicate.reduction_factor(&self.plan)?)
    }

    pub(crate) fn distinct_values(&self, field_name: &Element) -> common::DbResult<i32> {
        if self.predicate.equates_with_constant(field_name)?.is_some() {
            return Ok(1);
        } else {
            if let Some(second_field) = self.predicate.equates_with_field(field_name)? {
                return Ok(self
                    .plan
                    .distinct_values(field_name)?
                    .min(self.plan.distinct_values(&second_field)?));
            }
        }
        self.plan.distinct_values(field_name)
    }

    pub(crate) fn schema(&self) -> DbResult<Schema> {
        self.plan.schema()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SchemaBuilder;
    use crate::{
        mgr::metadata::MetadataMgr,
        predicate::{Expression, Term},
        tests::init,
        value::Value,
    };

    #[test]
    fn select() {
        let (_dir, tx) = init();
        let md = MetadataMgr::new(true, &tx).unwrap();
        let schema = SchemaBuilder::default()
            .add_int_field(Element::raw("id"))
            .build();
        md.create_table("test", schema.clone(), &tx).unwrap();
        tx.commit().unwrap();

        let table = Plan::table(&tx, "test".to_string(), &md).unwrap();

        let field = Element::raw("id");
        let value = Value::Integer(1);

        let predicate = predicate(field.clone(), value.clone());

        let select = Plan::select(Box::new(table), predicate);

        let mut select_scan = select.open().unwrap();

        for _ in 0..10 {
            for i in 0..10 {
                select_scan.insert().unwrap();
                select_scan.set_i32(&field, i).unwrap();
            }
        }

        assert_eq!(0, select.blocks_accessed().unwrap());
        assert_eq!(0, select.records_output().unwrap());
        assert_eq!(1, select.distinct_values(&field).unwrap());
        assert_eq!(schema, select.schema().unwrap());

        select_scan.before_first().unwrap();
        while select_scan.next_row().unwrap() {
            let found = select_scan.get_val(&field).unwrap();
            assert_eq!(value, found);
        }
    }

    fn predicate(field: Element, value: Value) -> Predicate {
        let expression1 = Expression::Field(field.clone());
        let expression2 = Expression::Value(value.clone());

        let term = Term::new(expression1, expression2);
        Predicate::new(term)
    }
}
