use crate::runner::Runner;
use common::DbResult;
use std::path::Path;

mod runner;
mod session;

pub fn run(port: u16, path: &Path) -> DbResult<()> {
    let runner = Runner::new(port, path)?;
    runner.run()
}
