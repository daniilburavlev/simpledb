use common::DbResult;
use engine::SimpleDB;
use engine::element::Element;
use engine::scan::Scan;
use protocol::{DbRequest, DbResponse, Frame};
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
            Ok(DbRequest::Execute(query)) => self.process_execute(stream, query, db),
            Ok(DbRequest::Next) => self.process_next(stream),
            Ok(DbRequest::GetField(field)) => self.process_get_field(stream, field),
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

    fn process_execute<W: Write>(
        &mut self,
        w: &mut W,
        query: String,
        db: &SimpleDB,
    ) -> DbResult<()> {
        let response = match db.execute(&self.tx, &query) {
            Ok(code) => DbResponse::Execute(code),
            Err(e) => DbResponse::Err(e.to_string()),
        };
        response.write(w)?;
        Ok(())
    }

    fn process_next<W: Write>(&mut self, w: &mut W) -> DbResult<()> {
        let Some(scan) = &self.scan else {
            let response = DbResponse::Err("query not executed".to_string());
            response.write(w)?;
            return Ok(());
        };
        let response = match scan.next() {
            Ok(next) => DbResponse::HasNext(next),
            Err(e) => DbResponse::Err(e.to_string()),
        };
        response.write(w)?;
        Ok(())
    }

    fn process_get_field<W: Write>(&mut self, w: &mut W, field: String) -> DbResult<()> {
        let Some(scan) = &self.scan else {
            let response = DbResponse::Err("query not executed".to_string());
            response.write(w)?;
            return Ok(());
        };
        let response = match scan.get_val(&Element::Raw(field)) {
            Ok(value) => DbResponse::Value(value),
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
