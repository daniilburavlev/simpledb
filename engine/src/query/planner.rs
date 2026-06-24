use std::{rc::Rc, sync::Arc};

use common::{DbResult, error::DbError};
use transaction::transaction::Transaction;

use crate::{
    plan::Plan,
    query::{
        command::{
            Command, DeleteData, IndexData, InsertData, QueryData, TableData, UpdateData, ViewData,
        },
        parser::Parser,
    },
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

    fn execute_create_index(&self, data: IndexData, tx: &Arc<Transaction>) -> DbResult<i32>;
}

pub struct Planner {
    query_planner: Rc<dyn QueryPlanner>,
    update_planner: Rc<dyn UpdatePlanner>,
}

impl Planner {
    pub fn new(query_planner: Rc<dyn QueryPlanner>, update_planner: Rc<dyn UpdatePlanner>) -> Self {
        Self {
            query_planner,
            update_planner,
        }
    }

    pub fn create_query_plan(&self, query: &str, tx: &Arc<Transaction>) -> DbResult<Rc<dyn Plan>> {
        let parser = Parser::new(query)?;
        let data = parser.query()?;
        if let Command::Query(data) = data {
            self.query_planner.create_plan(data, tx)
        } else {
            Err(DbError::other("expected select query"))
        }
    }

    pub fn execute_update(&self, query: &str, tx: &Arc<Transaction>) -> DbResult<i32> {
        let parser = Parser::new(query)?;
        match parser.update_cmd()? {
            Command::Insert(data) => self.update_planner.execute_insert(data, tx),
            Command::Delete(data) => self.update_planner.execute_delete(data, tx),
            Command::Update(data) => self.update_planner.execute_update(data, tx),
            Command::CreateTable(data) => self.update_planner.execute_create_table(data, tx),
            Command::CreateView(data) => self.update_planner.execute_create_view(data, tx),
            Command::CreateIndex(data) => self.update_planner.execute_create_index(data, tx),
            _ => Err(DbError::other("expected modify query")),
        }
    }
}
