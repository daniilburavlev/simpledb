use std::sync::Arc;

use common::{DbResult, error::DbError};
use transaction::transaction::Transaction;

use crate::{element::Element, metadata_mgr::MetadataMgr};

pub(crate) mod query;
pub(crate) mod update;

pub(crate) fn check_layout<'a, I>(
    md: &MetadataMgr,
    table: &str,
    fields: I,
    tx: &Arc<Transaction>,
) -> DbResult<()>
where
    I: IntoIterator<Item = &'a Element>,
{
    let layout = md.get_layout(table, tx)?;
    for field in fields {
        if !layout.schema().has_field(field) {
            return Err(DbError::FieldNotExists(field.to_string()));
        }
    }
    Ok(())
}
