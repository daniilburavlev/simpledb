use common::DbResult;
use engine::SimpleDB;
use engine::scan::Scan;
use protocol::{DbRequest, DbResponse};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::rc::Rc;
use std::sync::Arc;
use transaction::transaction::Transaction;

struct Session {
    tx: Arc<Transaction>,
    scan: Option<Rc<dyn Scan>>,
}

impl Session {
    fn new(tx: Arc<Transaction>) -> Self {
        Self { scan: None, tx }
    }

    fn process_request<S: Write + Read>(&mut self, stream: &mut S, db: &SimpleDB) -> DbResult<()> {
        match DbRequest::read(stream) {
            Ok(DbRequest::Query(query)) => self.process_query(stream, query, db),
            Err(e) => Err(e),
        }
    }

    fn process_query<W: Write>(&mut self, w: &mut W, query: String, db: &SimpleDB) -> DbResult<()> {
        let response = match db.query(&self.tx, &query) {
            Ok(scan) => {
                self.scan = Some(scan);
                DbResponse::Ok
            }
            Err(e) => DbResponse::Err(e.to_string()),
        };
        response.write(w)?;
        Ok(())
    }
}

pub(crate) fn handle_connection(mut stream: TcpStream, db: SimpleDB) -> DbResult<()> {
    let tx = db.get_tx()?;
    let mut session = Session::new(tx.clone());
    loop {
        if let Err(e) = session.process_request(&mut stream, &db) {
            tracing::error!("connection error: {}", e);
            break;
        }
    }
    tx.commit()?;
    Ok(())
}
