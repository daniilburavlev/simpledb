use std::{rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    plan::Plan,
    query::command::{DeleteData, InsertData, QueryData, TableData, UpdateData, ViewData},
};

pub trait QueryPlanner {
    fn create_plan(&self, data: QueryData, tx: &Arc<Transaction>) -> DbResult<Rc<dyn Plan>>;
}

pub trait UpdatePlanner {
    fn execute_insert(&self, data: InsertData, tx: &Arc<Transaction>) -> DbResult<i32>;

    fn execute_update(&self, data: UpdateData, tx: &Arc<Transaction>) -> DbResult<i32>;

    fn execute_delete(&self, data: DeleteData, tx: &Arc<Transaction>) -> DbResult<i32>;

    fn execute_create_table(&self, data: TableData, tx: &Arc<Transaction>) -> DbResult<i32>;

    fn execute_create_view(&self, data: ViewData, tx: &Arc<Transaction>) -> DbResult<i32>;

    fn execute_create_index(&self, data: InsertData, tx: &Arc<Transaction>) -> DbResult<i32>;
}
