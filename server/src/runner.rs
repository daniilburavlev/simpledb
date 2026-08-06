use crate::session;
use common::DbResult;
use engine::SimpleDB;
use std::net::TcpListener;
use std::path::Path;
use std::thread;

pub(crate) struct Runner {
    listener: TcpListener,
    db: SimpleDB,
}

impl Runner {
    pub(crate) fn new(port: u16, path: &Path) -> DbResult<Self> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        let db = SimpleDB::new(path)?;
        Ok(Self { listener, db })
    }

    pub(crate) fn run(self) -> DbResult<()> {
        loop {
            let (stream, addr) = self.listener.accept()?;
            let db = self.db.clone();
            tracing::debug!("new connection: {}", addr);
            thread::spawn(move || {
                if let Err(e) = session::handle_connection(stream, db) {
                    tracing::error!("{}", e);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::value::Value;
    use protocol::{DbRequest, DbResponse, Frame};
    use rand::RngExt;
    use std::net::TcpStream;
    use tempfile::tempdir;

    #[test]
    fn runner_next_get_table() {
        let dir = tempdir().unwrap();
        let mut rng = rand::rng();
        let port = rng.random::<u16>();
        let runner = Runner::new(port, dir.path()).unwrap();
        thread::spawn(move || {
            runner.run().unwrap();
        });

        let mut stream = TcpStream::connect(format!("0.0.0.0:{}", port)).unwrap();

        let create = DbRequest::Execute("CREATE TABLE users(id INT, name VARCHAR(16))".to_string());
        create.write(&mut stream).unwrap();

        let response = DbResponse::read(&mut stream).unwrap();
        assert_eq!(response, DbResponse::Execute(0));

        let insert =
            DbRequest::Execute("INSERT INTO users(id, name) VALUES(1, 'User')".to_string());
        insert.write(&mut stream).unwrap();

        let response = DbResponse::read(&mut stream).unwrap();
        assert_eq!(response, DbResponse::Execute(1));

        let select = DbRequest::Query("SELECT id, name FROM users WHERE id=1".to_string());
        select.write(&mut stream).unwrap();

        let response = DbResponse::read(&mut stream).unwrap();
        assert_eq!(response, DbResponse::Ok);

        let select = DbRequest::Next;
        select.write(&mut stream).unwrap();

        let response = DbResponse::read(&mut stream).unwrap();
        assert_eq!(response, DbResponse::HasNext(true));

        let get_int = DbRequest::GetField("id".to_string());
        get_int.write(&mut stream).unwrap();

        let response = DbResponse::read(&mut stream).unwrap();
        assert_eq!(response, DbResponse::Value(Value::Integer(1)));

        let get_string = DbRequest::GetField("name".to_string());
        get_string.write(&mut stream).unwrap();

        let response = DbResponse::read(&mut stream).unwrap();
        assert_eq!(
            response,
            DbResponse::Value(Value::Varchar("User".to_string()))
        );
    }
}
