use common::DbResult;
use common::error::DbError;
use engine::value::{INTEGER_TYPE, VARCHAR_TYPE, Value};
use std::io::{Read, Write};

const TYPE_SIZE: usize = 1;
const CODE_SIZE: usize = 4;
const LEN_SIZE: usize = 2;

const REQUEST_QUERY: u8 = b'q';
const REQUEST_EXEC: u8 = b'e';
const REQUEST_NEXT: u8 = b'n';
const REQUEST_FIELD: u8 = b'v';

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbRequest {
    Query(String),
    Execute(String),
    Next,
    GetField(String),
}

impl Frame for DbRequest {
    fn read<R: Read>(read: &mut R) -> DbResult<Self> {
        let mut request_type = [0u8];
        read.read_exact(&mut request_type)?;
        match request_type[0] {
            REQUEST_QUERY => {
                let query = read_str(read)?;
                Ok(DbRequest::Query(query))
            }
            REQUEST_EXEC => {
                let query = read_str(read)?;
                Ok(DbRequest::Execute(query))
            }
            REQUEST_NEXT => Ok(DbRequest::Next),
            REQUEST_FIELD => {
                let field = read_str(read)?;
                Ok(DbRequest::GetField(field))
            }
            _ => Err(DbError::InvalidValue),
        }
    }

    fn write<W: Write>(&self, w: &mut W) -> DbResult<usize> {
        let mut write = 0;
        match self {
            DbRequest::Query(query) => {
                let buffer = str_buffer(REQUEST_QUERY, query);
                w.write_all(&buffer)?;
                write += buffer.len();
            }
            DbRequest::Execute(query) => {
                let buffer = str_buffer(REQUEST_EXEC, query);
                w.write_all(&buffer)?;
                write += buffer.len();
            }
            DbRequest::Next => {
                let buffer = [REQUEST_NEXT];
                w.write_all(&buffer)?;
                write += buffer.len();
            }
            DbRequest::GetField(field) => {
                let buffer = str_buffer(REQUEST_FIELD, field);
                w.write_all(&buffer)?;
                write += buffer.len();
            }
        }
        Ok(write)
    }
}

const RESPONSE_OK: u8 = b'o';
const RESPONSE_ERR: u8 = b'e';
const RESPONSE_EXEC: u8 = b'x';
const RESPONSE_HAS_NEXT: u8 = b'n';
const RESPONSE_VALUE: u8 = b'v';

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbResponse {
    Ok,
    Err(String),
    Execute(i32),
    HasNext(bool),
    Value(Value),
}

impl Frame for DbResponse {
    fn read<R: Read>(read: &mut R) -> DbResult<Self> {
        let mut request_type = [0u8];
        read.read_exact(&mut request_type)?;
        match request_type[0] {
            RESPONSE_OK => Ok(DbResponse::Ok),
            RESPONSE_ERR => {
                let err = read_str(read)?;
                Ok(Self::Err(err))
            }
            RESPONSE_EXEC => {
                let code = read_i32(read)?;
                Ok(Self::Execute(code))
            }
            RESPONSE_HAS_NEXT => {
                let mut next = [0u8];
                read.read_exact(&mut next)?;
                Ok(Self::HasNext(next[0] == 1))
            }
            RESPONSE_VALUE => {
                let mut value = [0u8];
                read.read_exact(&mut value)?;
                match value[0] {
                    VARCHAR_TYPE => {
                        let value = read_str(read)?;
                        Ok(Self::Value(Value::Varchar(value)))
                    }
                    INTEGER_TYPE => {
                        let value = read_i32(read)?;
                        Ok(Self::Value(Value::Integer(value)))
                    }
                    _ => Err(DbError::InvalidValue),
                }
            }
            _ => Err(DbError::InvalidValue),
        }
    }

    fn write<W: Write>(&self, w: &mut W) -> DbResult<usize> {
        let mut write = 0;
        match self {
            DbResponse::Ok => {
                w.write_all(&[RESPONSE_OK])?;
                write += TYPE_SIZE;
            }
            DbResponse::Err(err) => {
                let buffer = str_buffer(RESPONSE_ERR, err);
                w.write_all(&buffer)?;
                write += buffer.len();
            }
            DbResponse::Execute(code) => {
                write += write_i32(w, RESPONSE_EXEC, *code)?;
            }
            DbResponse::HasNext(code) => {
                let buffer = vec![RESPONSE_HAS_NEXT, if *code { 1 } else { 0 }];
                w.write_all(&buffer)?;
                write += buffer.len();
            }
            DbResponse::Value(value) => match value {
                Value::Integer(value) => {
                    let mut buffer = vec![0u8; TYPE_SIZE + TYPE_SIZE + CODE_SIZE];
                    buffer[0] = RESPONSE_VALUE;
                    buffer[1] = INTEGER_TYPE;
                    buffer[2 * TYPE_SIZE..].copy_from_slice(&value.to_be_bytes());
                    w.write_all(&buffer)?;
                }
                Value::Varchar(value) => {
                    let len = value.len();
                    let mut buffer = vec![0u8; TYPE_SIZE + TYPE_SIZE + LEN_SIZE + len];
                    buffer[0] = RESPONSE_VALUE;
                    buffer[1] = VARCHAR_TYPE;
                    buffer[2..2 + LEN_SIZE].copy_from_slice((len as u16).to_le_bytes().as_ref());
                    buffer[2 + LEN_SIZE..2 + LEN_SIZE + len].copy_from_slice(value.as_ref());
                    w.write_all(&buffer)?;
                }
            },
        }
        Ok(write)
    }
}

pub trait Frame: PartialEq + std::fmt::Debug + Sized {
    fn read<R: Read>(read: &mut R) -> DbResult<Self>;

    fn write<W: Write>(&self, w: &mut W) -> DbResult<usize>;
}

fn str_buffer(t: u8, value: &String) -> Vec<u8> {
    let len = value.len();
    let mut buffer = vec![0u8; TYPE_SIZE + LEN_SIZE + len];
    buffer[0] = t;
    buffer[TYPE_SIZE..TYPE_SIZE + LEN_SIZE].copy_from_slice((len as u16).to_be_bytes().as_ref());
    buffer[TYPE_SIZE + LEN_SIZE..].copy_from_slice(value.as_bytes());
    buffer
}

fn write_i32<W: Write>(w: &mut W, t: u8, value: i32) -> DbResult<usize> {
    let mut buffer = [0u8; TYPE_SIZE + CODE_SIZE];
    buffer[0] = t;
    buffer[TYPE_SIZE..TYPE_SIZE + CODE_SIZE].copy_from_slice(&value.to_be_bytes());
    w.write_all(&buffer)?;
    Ok(TYPE_SIZE + CODE_SIZE)
}

fn read_str<R: Read>(r: &mut R) -> DbResult<String> {
    let mut len = [0u8; LEN_SIZE];
    r.read_exact(&mut len)?;
    let len = u16::from_be_bytes(len);
    let mut value = vec![0u8; len as usize];
    r.read_exact(&mut value)?;
    Ok(String::from_utf8_lossy(&value).to_string())
}

fn read_i32<R: Read>(r: &mut R) -> DbResult<i32> {
    let mut value = [0u8; CODE_SIZE];
    r.read_exact(&mut value)?;
    Ok(i32::from_be_bytes(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, BufWriter};

    #[test]
    fn write_read_query_request() {
        let query = DbRequest::Query("SELECT * FROM users".to_string());
        write_read(query, 22);
    }

    #[test]
    fn write_read_execute_request() {
        let query = DbRequest::Execute("CREATE TABLE users(id INT, name VARCHAR(256))".to_string());
        write_read(query, 48);
    }

    #[test]
    fn write_read_next_request() {
        let query = DbRequest::Next;
        write_read(query, 1);
    }

    #[test]
    fn write_read_get_field_request() {
        let query = DbRequest::GetField("id".to_string());
        write_read(query, 5);
    }

    #[test]
    fn write_read_ok_response() {
        let query = DbResponse::Ok;
        write_read(query, 1);
    }

    #[test]
    fn write_read_err_response() {
        let query = DbResponse::Err("relation 'users' not exists".to_string());
        write_read(query, 30);
    }

    #[test]
    fn write_read_execute_response() {
        let query = DbResponse::Execute(0);
        write_read(query, 5);
    }

    fn write_read<F: Frame>(f: F, size: usize) {
        let mut buffer = Vec::new();
        let write = {
            let mut write = BufWriter::new(&mut buffer);
            let w = f.write(&mut write).unwrap();
            write.flush().unwrap();
            assert_eq!(w, size);
            w
        };
        let mut read = BufReader::new(&buffer[..write]);
        let read = Frame::read(&mut read).unwrap();
        assert_eq!(f, read);
    }
}
