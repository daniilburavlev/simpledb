use std::{collections::HashMap, fmt::Debug, sync::RwLock};

use common::{DbResult, error::DbError};

use crate::field_info::FieldInfo;

#[derive(Debug)]
struct SchemaLock {
    fields: Vec<String>,
    infos: HashMap<String, FieldInfo>,
}

impl SchemaLock {
    fn new() -> Self {
        Self {
            fields: Vec::new(),
            infos: HashMap::new(),
        }
    }

    fn add_field(&mut self, fieldname: String, field_info: FieldInfo) {
        self.fields.push(fieldname.clone());
        self.infos.insert(fieldname, field_info);
    }

    fn add_int_field(&mut self, fieldname: String) {
        self.add_field(fieldname, FieldInfo::Integer);
    }

    fn add_string_field(&mut self, fieldname: String, length: i32) {
        self.add_field(fieldname, FieldInfo::Varchar(length));
    }

    fn add(&mut self, fieldname: String, schema: &Self) {
        if let Some(info) = schema.info(&fieldname) {
            self.add_field(fieldname, info);
        }
    }

    fn add_all(&mut self, schema: &Self) {
        for (field, info) in &schema.infos {
            self.add_field(field.clone(), info.clone());
        }
    }

    fn info(&self, fieldname: &str) -> Option<FieldInfo> {
        self.infos.get(fieldname).cloned()
    }
}

pub struct Schema {
    lock: RwLock<SchemaLock>,
}

impl Debug for Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let read = self.lock.read().unwrap();
        write!(f, "Schema {{ {:?} }}", &*read)
    }
}

impl Schema {
    pub fn add_field(&self, fieldname: String, info: FieldInfo) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.add_field(fieldname, info);
        Ok(())
    }

    pub fn add_int_field(&self, fieldname: String) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.add_int_field(fieldname);
        Ok(())
    }

    pub fn add_string_field(&self, fieldname: String, length: i32) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.add_string_field(fieldname, length);
        Ok(())
    }

    pub fn add(&self, fieldname: String, schema: &Self) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        let read = schema.lock.read().map_err(DbError::lock)?;
        write.add(fieldname, &read);
        Ok(())
    }

    pub fn add_all(&self, schema: &Self) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        let read = schema.lock.read().map_err(DbError::lock)?;
        write.add_all(&read);
        Ok(())
    }

    pub fn has_field(&self, fieldname: &str) -> DbResult<bool> {
        let read = self.lock.read().map_err(DbError::lock)?;
        Ok(read.infos.contains_key(fieldname))
    }

    pub fn info(&self, fieldname: &str) -> DbResult<Option<FieldInfo>> {
        let read = self.lock.read().map_err(DbError::lock)?;
        Ok(read.info(fieldname))
    }

    pub fn fields(&self) -> DbResult<Vec<(String, FieldInfo)>> {
        let read = self.lock.read().map_err(DbError::lock)?;
        let mut result = vec![];
        for field in &read.fields {
            if let Some(info) = read.infos.get(field) {
                result.push((field.clone(), info.clone()));
            }
        }
        Ok(result)
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            lock: RwLock::new(SchemaLock::new()),
        }
    }
}
