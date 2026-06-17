use common::DbResult;

pub trait Index {
    fn before_first(&self) -> DbResult<()>;

    fn next(&self) -> DbResult<bool>;

    fn get_data_rid(&self) -> DbResult<Index>
}
