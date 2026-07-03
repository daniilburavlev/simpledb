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
    use protocol::{DbRequest, DbResponse};
    use rand::RngExt;
    use std::net::TcpStream;
    use tempfile::tempdir;

    #[test]
    fn runner_query() {
        let dir = tempdir().unwrap();
        let mut rng = rand::rng();
        let port = rng.random::<u16>();
        let runner = Runner::new(port, dir.path()).unwrap();
        thread::spawn(move || {
            runner.run().unwrap();
        });

        let mut stream = TcpStream::connect(format!("0.0.0.0:{}", port)).unwrap();

        let query = DbRequest::Query("select id, name from users".to_string());
        query.write(&mut stream).unwrap();
        let response = DbResponse::read(&mut stream).unwrap();
        assert_eq!(
            response,
            DbResponse::Err("relation 'users' not exists".to_string())
        )
    }
}
