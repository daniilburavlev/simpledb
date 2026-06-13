use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use common::{DbResult, error::DbError};
use transaction::transaction::Transaction;

use crate::{
    layout::Layout,
    scan::{Scan, UpdateScan, table_scan::TableScan},
    table_mgr::{TABLE_NAME, TableMgr},
};

#[derive(Clone, Debug)]
pub struct StatInfo {
    num_blocks: i32,
    num_recs: u16,
}

impl StatInfo {
    pub fn new(num_blocks: i32, num_recs: u16) -> Self {
        Self {
            num_blocks,
            num_recs,
        }
    }

    pub fn block_accessed(&self) -> i32 {
        self.num_blocks
    }

    pub fn records_output(&self) -> i32 {
        self.num_recs as i32
    }

    pub fn distinct_values(&self) -> i32 {
        1 + (self.num_recs / 3) as i32
    }
}

struct StatMgrLock {
    table_mgr: Arc<TableMgr>,
    table_stats: HashMap<String, StatInfo>,
    num_calls: u64,
}

impl StatMgrLock {
    fn new(table_mgr: &Arc<TableMgr>) -> Self {
        Self {
            table_mgr: Arc::clone(table_mgr),
            table_stats: HashMap::new(),
            num_calls: 0,
        }
    }

    fn get_stat_info(
        &mut self,
        table_name: &str,
        layout: &Arc<Layout>,
        tx: &Arc<Transaction>,
    ) -> DbResult<StatInfo> {
        self.num_calls += 1;
        if self.num_calls > 100 {
            self.refresh_statisic(tx)?;
        }
        if let Some(stat) = self.table_stats.get(table_name).cloned() {
            Ok(stat)
        } else {
            let stat = self.calc_table_stats(table_name, layout, tx)?;
            self.table_stats
                .insert(table_name.to_string(), stat.clone());
            Ok(stat)
        }
    }

    fn refresh_statisic(&mut self, tx: &Arc<Transaction>) -> DbResult<()> {
        self.table_stats.clear();
        self.num_calls = 0;
        let layout = Arc::new(self.table_mgr.get_layout(TABLE_NAME, tx)?);
        let ts = TableScan::new(tx, TABLE_NAME, &layout)?;
        while ts.next()? {
            let table_name = ts.get_string(TABLE_NAME)?;
            let layout = self.table_mgr.get_layout(&table_name, tx)?;
            let stat = self.calc_table_stats(&table_name, &Arc::new(layout), tx)?;
            self.table_stats.insert(table_name, stat);
        }
        ts.close()
    }

    fn calc_table_stats(
        &self,
        table_name: &str,
        layout: &Arc<Layout>,
        tx: &Arc<Transaction>,
    ) -> DbResult<StatInfo> {
        let mut num_recs = 0;
        let mut num_blocks = 0;
        let ts = TableScan::new(tx, table_name, layout)?;
        while ts.next()? {
            num_recs += 1;
            num_blocks = ts.get_rid()?.block_num() + 1;
        }
        ts.close()?;
        Ok(StatInfo::new(num_blocks, num_recs))
    }
}

pub struct StatMgr {
    lock: RwLock<StatMgrLock>,
}

impl StatMgr {
    pub fn new(table_mgr: &Arc<TableMgr>, tx: &Arc<Transaction>) -> DbResult<Self> {
        let mut lock = StatMgrLock::new(table_mgr);
        lock.refresh_statisic(tx)?;
        Ok(Self {
            lock: RwLock::new(lock),
        })
    }

    pub fn get_stat_info(
        &self,
        table_name: &str,
        layout: &Arc<Layout>,
        tx: &Arc<Transaction>,
    ) -> DbResult<StatInfo> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.get_stat_info(table_name, layout, tx)
    }
}
