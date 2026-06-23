use std::{cmp::Ordering, rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    plan::Plan,
    scan::{
        Scan,
        sort::{RecordComparator, SortScan},
    },
    schema::Schema,
    temp::{TempTable, meterialize_plan::MaterializePlan},
};

pub struct SortPlan {
    plan: Rc<dyn Plan>,
    tx: Arc<Transaction>,
    schema: Arc<Schema>,
    comp: RecordComparator,
}

impl SortPlan {
    pub fn new(
        tx: &Arc<Transaction>,
        plan: &Rc<dyn Plan>,
        sort_fields: Vec<String>,
    ) -> DbResult<Self> {
        let schema = plan.schema()?;
        Ok(Self {
            plan: Rc::clone(plan),
            tx: Arc::clone(tx),
            schema,
            comp: RecordComparator::new(sort_fields),
        })
    }

    fn split_into_runs(&self, source: &Rc<dyn Scan>) -> DbResult<Vec<TempTable>> {
        let mut temps = vec![];
        source.before_first()?;
        if !source.next()? {
            return Ok(temps);
        }
        let mut current_temp = TempTable::new(&self.tx, &self.schema)?;
        let mut current_scan = current_temp.open()?;
        temps.push(current_temp);
        while self.copy(source, &current_scan)? {
            if self.comp.compare(source, &current_scan)? == Ordering::Less {
                current_scan.close()?;
                current_temp = TempTable::new(&self.tx, &self.schema)?;
                temps.push(current_temp.clone());
                current_scan = current_temp.open()?;
            }
        }
        current_scan.close()?;
        Ok(temps)
    }

    fn do_merge_iteration(&self, mut runs: Vec<TempTable>) -> DbResult<Vec<TempTable>> {
        let mut result = vec![];
        while runs.len() > 1 {
            let p1 = runs.remove(0);
            let p2 = runs.remove(0);
            result.push(self.merge_two_runs(p1, p2)?);
        }
        if runs.len() == 1 {
            result.push(runs.first().cloned().unwrap());
        }
        Ok(result)
    }

    fn merge_two_runs(&self, p1: TempTable, p2: TempTable) -> DbResult<TempTable> {
        let src1 = p1.open()?;
        let src2 = p2.open()?;
        let result = TempTable::new(&self.tx, &self.schema)?;
        let dest = result.open()?;

        let mut has_more1 = src1.next()?;
        let mut has_more2 = src2.next()?;
        while has_more1 && has_more2 {
            if self.comp.compare(&src1, &src2)? == Ordering::Less {
                has_more1 = self.copy(&src1, &dest)?;
            } else {
                has_more2 = self.copy(&src2, &dest)?;
            }
        }

        if has_more1 {
            while has_more1 {
                has_more1 = self.copy(&src1, &dest)?;
            }
        } else {
            while has_more2 {
                has_more2 = self.copy(&src2, &dest)?;
            }
        }
        src1.close()?;
        src2.close()?;
        dest.close()?;
        Ok(result)
    }

    fn copy(&self, source: &Rc<dyn Scan>, dest: &Rc<dyn Scan>) -> DbResult<bool> {
        dest.insert()?;
        for (field, _) in self.schema.fields()? {
            dest.set_val(&field, source.get_val(&field)?)?;
        }
        source.next()
    }
}

impl Plan for SortPlan {
    fn open(&self) -> DbResult<Rc<dyn Scan>> {
        let scan = self.plan.open()?;
        let mut runs = self.split_into_runs(&scan)?;
        scan.close()?;
        while runs.len() > 2 {
            runs = self.do_merge_iteration(runs)?;
        }
        Ok(Rc::new(SortScan::new(runs, self.comp.clone())?))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        let mp = MaterializePlan::new(&self.plan, &self.tx);
        mp.blocks_accessed()
    }

    fn records_output(&self) -> DbResult<i32> {
        self.plan.records_output()
    }

    fn distinct_values(&self, field_name: &str) -> DbResult<i32> {
        self.plan.distinct_values(field_name)
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        Ok(self.schema.clone())
    }
}
