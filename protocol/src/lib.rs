use common::DbResult;
use common::error::DbError;
use std::io::{Read, Write};

const TYPE_SIZE: usize = 1;
const LEN_SIZE: usize = 2;

const REQUEST_QUERY: u8 = b'q';

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbRequest {
    Query(String),
}

impl DbRequest {
    pub fn read<R: Read>(read: &mut R) -> DbResult<Self> {
        let mut request_type = [0u8];
        read.read_exact(&mut request_type)?;
        match request_type[0] {
            REQUEST_QUERY => {
                let mut len = [0u8; 2];
                read.read_exact(&mut len)?;
                let len = u16::from_be_bytes(len);
                let mut query = vec![0u8; len as usize];
                read.read_exact(query.as_mut_slice())?;
                let query = String::from_utf8_lossy(&query);
                Ok(DbRequest::Query(query.to_string()))
            }
            _ => Err(DbError::InvalidValue),
        }
    }

    pub fn write<W: Write>(&self, w: &mut W) -> DbResult<usize> {
        let mut write = 0;
        match self {
            DbRequest::Query(query) => {
                let len = query.len();
                let mut buffer = vec![0u8; TYPE_SIZE + LEN_SIZE + len];
                let len = len as u16;
                let mut offset = 0;
                buffer[offset] = REQUEST_QUERY;
                offset += TYPE_SIZE;
                buffer[offset..offset + LEN_SIZE].copy_from_slice(len.to_be_bytes().as_ref());
                offset += LEN_SIZE;
                buffer[offset..].copy_from_slice(query.as_bytes());
                w.write_all(&buffer)?;
                write += buffer.len();
            }
        }
        Ok(write)
    }
}

const RESPONSE_OK: u8 = b'o';
const RESPONSE_ERR: u8 = b'e';

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbResponse {
    Ok,
    Err(String),
}

impl DbResponse {
    pub fn read<R: Read>(read: &mut R) -> DbResult<Self> {
        let mut request_type = [0u8];
        read.read_exact(&mut request_type)?;
        match request_type[0] {
            RESPONSE_OK => Ok(DbResponse::Ok),
            RESPONSE_ERR => {
                let mut len = [0u8; 2];
                read.read_exact(&mut len)?;
                let len = u16::from_be_bytes(len);
                let mut err = vec![0u8; len as usize];
                read.read_exact(&mut err)?;
                let err = String::from_utf8_lossy(&err);
                Ok(Self::Err(err.to_string()))
            }
            _ => Err(DbError::InvalidValue),
        }
    }

    pub fn write<W: Write>(&self, w: &mut W) -> DbResult<usize> {
        let mut write = 0;
        match self {
            DbResponse::Ok => {
                w.write_all(&[RESPONSE_OK])?;
                write += TYPE_SIZE;
            },
            DbResponse::Err(err) => {
                let len = err.len();
                let mut buffer = vec![0u8; TYPE_SIZE + LEN_SIZE + len];
                buffer[0] = RESPONSE_ERR;
                buffer[TYPE_SIZE..TYPE_SIZE + LEN_SIZE].copy_from_slice((len as u16).to_be_bytes().as_ref());
                buffer[TYPE_SIZE + LEN_SIZE..].copy_from_slice(err.as_bytes());
                w.write_all(&buffer)?;
                write += buffer.len();
            }
        }
        Ok(write)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, BufWriter};

    #[test]
    fn write_read_query_request() {
        let query = DbRequest::Query("SELECT * FROM users".to_string());
        let mut buffer = Vec::new();
        let write = {
            let mut write = BufWriter::new(&mut buffer);
            let w = query.write(&mut write).unwrap();
            write.flush().unwrap();
            assert_eq!(w, 22);
            w
        };

        let mut read = BufReader::new(&buffer[..write]);
        let read = DbRequest::read(&mut read).unwrap();
        assert_eq!(query, read);
    }

    #[test]
    fn write_read_ok_response() {
        let query = DbResponse::Ok;
        let mut buffer = Vec::new();
        let write = {
            let mut write = BufWriter::new(&mut buffer);
            let w = query.write(&mut write).unwrap();
            write.flush().unwrap();
            assert_eq!(w, 1);
            w
        };

        let mut read = BufReader::new(&buffer[..write]);
        let read = DbResponse::read(&mut read).unwrap();
        assert_eq!(query, read);
    }
}
