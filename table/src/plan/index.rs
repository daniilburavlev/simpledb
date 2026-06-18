use std::{rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    constant::Constant,
    index_mgr::IndexInfo,
    metadata_mgr::MetadataMgr,
    plan::{Plan, select::SelectPlan, table::TablePlan},
    query::{
        command::{DeleteData, IndexData, InsertData, TableData, UpdateData, ViewData},
        planner::UpdatePlanner,
    },
    scan::index::{IndexJoinScan, IndexSelectScan},
    schema::Schema,
};

pub struct IndexSelectPlan {
    plan: Rc<dyn Plan>,
    index: IndexInfo,
    value: Constant,
}

impl IndexSelectPlan {
    pub fn new(plan: &Rc<dyn Plan>, index: IndexInfo, value: Constant) -> Self {
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

    fn distinct_values(&self, field_name: &str) -> DbResult<i32> {
        Ok(self.index.distinct_values(field_name))
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        self.plan.schema()
    }
}

pub struct IndexJoinPlan {
    p1: Rc<dyn Plan>,
    p2: Rc<dyn Plan>,
    index: IndexInfo,
    field: String,
    schema: Arc<Schema>,
}

impl IndexJoinPlan {
    pub fn new(
        p1: &Rc<dyn Plan>,
        p2: &Rc<dyn Plan>,
        index: IndexInfo,
        field: String,
    ) -> DbResult<Self> {
        let schema = Arc::new(Schema::default());
        let s1 = p1.schema()?;
        let s2 = p2.schema()?;
        schema.add_all(&s1)?;
        schema.add_all(&s2)?;
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
        Ok(Rc::new(IndexJoinScan::new(&s, &idx, &self.field, &ts)?))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        Ok(self.p1.blocks_accessed()?
            + (self.p1.records_output()? * self.index.block_accessed()?)
            + self.records_output()?)
    }

    fn records_output(&self) -> DbResult<i32> {
        Ok(self.p1.records_output()? * self.index.records_output())
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

pub struct IndexUpdatePlanner {
    mg: Arc<MetadataMgr>,
}

impl UpdatePlanner for IndexUpdatePlanner {
    fn execute_insert(&self, data: InsertData, tx: &Arc<Transaction>) -> DbResult<i32> {
        let table = data.table;
        let indexes = self.mg.get_index_info(&table, tx)?;
        let plan = TablePlan::new(tx, table, &self.mg)?;

        let s = plan.open()?;
        s.insert()?;
        let rid = s.get_rid()?;

        for (field, value) in data.fields.iter().zip(data.values) {
            tracing::debug!("Modify field: {} {}", field, value);
            s.set_val(field, value.clone())?;

            if let Some(info) = indexes.get(field) {
                let index = info.open()?;
                index.insert(value, rid.clone())?;
                index.close()?;
            }
        }
        s.close()?;
        Ok(1)
    }

    fn execute_update(
        &self,
        data: UpdateData,
        tx: &Arc<Transaction>,
    ) -> DbResult<i32> {
        let table = data.table;
        let field = data.field;
        let index = if let Some(info) = self.mg.get_index_info(&table, tx)?.get(&field) {
            Some(info.open()?)
        } else {
            None
        };
        let plan = TablePlan::new(tx, table, &self.mg)?;
        let plan = SelectPlan::new(Rc::new(plan), data.predicate);
        let s = plan.open()?;
        let mut count = 0;
        while s.next()? {
            let new_val = data.value.evaluate(&s)?;
            let oldval = s.get_val(&field)?;
            s.set_val(&field, new_val.clone())?;
            if let Some(index) = &index {
                let rid = s.get_rid()?;
                index.delete(oldval, rid.clone())?;
                index.delete(new_val, rid.clone())?;
            }
            count += 1;
        }
        if let Some(index) = index {
            index.close()?;
        }
        s.close()?;
        Ok(count)
    }

    fn execute_delete(
        &self,
        data: DeleteData,
        tx: &Arc<Transaction>,
    ) -> DbResult<i32> {
        let table = data.name;
        let indexes = self.mg.get_index_info(&table, tx)?;
        let plan = TablePlan::new(tx, table, &self.mg)?;
        let plan = SelectPlan::new(Rc::new(plan), data.predicate);
        let s = plan.open()?;
        let mut count = 0;
        while s.next()? {
            let rid = s.get_rid()?;
            for (field, value) in indexes.iter() {
                let val = s.get_val(field)?;
                let index = value.open()?;
                index.delete(val, rid.clone())?;
                index.close()?;
            }
            s.delete()?;
            count += 1;
        }
        s.close()?;
        Ok(count)
    }

    fn execute_create_table(
        &self,
        data: TableData,
        tx: &Arc<transaction::transaction::Transaction>,
    ) -> DbResult<i32> {
        self.mg
            .create_table(&data.name, &Arc::new(data.schema), tx)?;
        Ok(0)
    }

    fn execute_create_view(
        &self,
        data: ViewData,
        tx: &Arc<Transaction>,
    ) -> DbResult<i32> {
        self.mg
            .create_view(&data.name, &data.query.to_string(), tx)?;
        Ok(0)
    }

    fn execute_create_index(
        &self,
        data: IndexData,
        tx: &Arc<Transaction>,
    ) -> DbResult<i32> {
        self.mg
            .create_index(&data.index, &data.table, &data.field, tx)?;
        Ok(0)
    }
}
