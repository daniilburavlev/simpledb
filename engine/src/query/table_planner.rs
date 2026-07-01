use std::{collections::HashMap, rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::element::Element;
use crate::schema_mapping::SchemaMapping;
use crate::{
    index_mgr::IndexInfo,
    metadata_mgr::MetadataMgr,
    plan::{
        Plan,
        index::{IndexJoinPlan, IndexSelectPlan},
        multibuffer::MultiBufferProductPlan,
        select::SelectPlan,
        table::TablePlan,
    },
    predicate::Predicate,
    schema::Schema,
};

pub(crate) struct TablePlanner {
    plan: Rc<TablePlan>,
    predicate: Predicate,
    schema: Schema,
    indexes: HashMap<Element, IndexInfo>,
    tx: Arc<Transaction>,
    mapping: SchemaMapping,
}

impl TablePlanner {
    pub(crate) fn new(
        table: Element,
        predicate: Predicate,
        tx: &Arc<Transaction>,
        md: &MetadataMgr,
        mapping: SchemaMapping,
    ) -> DbResult<Self> {
        let plan = TablePlan::new(tx, table.to_string(), md)?;
        Ok(Self {
            predicate,
            tx: Arc::clone(tx),
            schema: plan.schema()?,
            plan: Rc::new(plan),
            indexes: md.get_index_info(table.as_raw()?, tx)?,
            mapping,
        })
    }

    pub(crate) fn make_join_plan(&self, current: &Rc<dyn Plan>) -> DbResult<Option<Rc<dyn Plan>>> {
        let current_schema = current.schema()?;
        if self
            .predicate
            .join_sub_pred(&self.schema, &current_schema)?
            .is_none()
        {
            return Ok(None);
        }
        if let Some(p) = self.make_index_join(current, &current_schema)? {
            Ok(Some(p))
        } else {
            Ok(Some(self.make_product_join(current, &current_schema)?))
        }
    }

    pub(crate) fn make_select_plan(&self) -> DbResult<Rc<dyn Plan>> {
        let p = if let Some(p) = self.make_index_select()? {
            p
        } else {
            self.plan.clone()
        };
        self.add_select_pred(&p)
    }

    pub(crate) fn make_product_plan(&self, current: &Rc<dyn Plan>) -> DbResult<Rc<dyn Plan>> {
        let plan: Rc<dyn Plan> = self.plan.clone();
        let p = self.add_select_pred(&plan)?;
        Ok(Rc::new(MultiBufferProductPlan::new(&self.tx, current, &p)?))
    }

    pub(crate) fn make_index_select(&self) -> DbResult<Option<Rc<dyn Plan>>> {
        for (field, info) in &self.indexes {
            if let Some(value) = self.predicate.equates_with_constant(field)? {
                let plan: Rc<dyn Plan> = self.plan.clone();
                return Ok(Some(Rc::new(IndexSelectPlan::new(
                    &plan,
                    info.clone(),
                    value,
                ))));
            }
        }
        Ok(None)
    }

    fn make_index_join(
        &self,
        current: &Rc<dyn Plan>,
        schema: &Schema,
    ) -> DbResult<Option<Rc<dyn Plan>>> {
        for (field, info) in &self.indexes {
            if let Some(outer_field) = self.predicate.equates_with_field(field)?
                && self.schema.has_field(field)
            {
                let plan: Rc<dyn Plan> = self.plan.clone();
                let p: Rc<dyn Plan> = Rc::new(IndexJoinPlan::new(
                    current,
                    &plan,
                    info.clone(),
                    outer_field,
                )?);
                let p = self.add_select_pred(&p)?;
                return Ok(Some(self.add_join_pred(&p, schema)?));
            }
        }
        Ok(None)
    }

    fn make_product_join(&self, current: &Rc<dyn Plan>, schema: &Schema) -> DbResult<Rc<dyn Plan>> {
        let p = self.make_product_plan(current)?;
        self.add_join_pred(&p, schema)
    }

    fn add_select_pred(&self, p: &Rc<dyn Plan>) -> DbResult<Rc<dyn Plan>> {
        let select_predicate = self.predicate.select_sub_pred(&self.schema, &self.mapping)?;
        Ok(Rc::new(SelectPlan::new(p.clone(), select_predicate)))
    }

    fn add_join_pred(&self, p: &Rc<dyn Plan>, schema: &Schema) -> DbResult<Rc<dyn Plan>> {
        if let Some(join_predicate) = self.predicate.join_sub_pred(schema, &self.schema)? {
            Ok(Rc::new(SelectPlan::new(p.clone(), join_predicate)))
        } else {
            Ok(p.clone())
        }
    }
}
