use crate::element::Element;
use crate::scan::Scan;
use crate::schema::{Schema, SchemaBuilder};
use crate::schema_mapping::SchemaMapping;
use common::DbResult;
use common::error::DbError;
use std::{collections::HashSet, rc::Rc};

pub(crate) struct ProjectScan {
    scan: Rc<dyn Scan>,
    fields: HashSet<Element>,
    mapping: SchemaMapping,
}

impl ProjectScan {
    pub(crate) fn new(
        scan: Rc<dyn Scan>,
        table_fields: HashSet<Element>,
        mapping: SchemaMapping,
    ) -> Self {
        let mut fields = HashSet::new();
        for field in table_fields {
            fields.insert(field);
        }
        Self {
            scan,
            fields,
            mapping,
        }
    }

    fn get_field(&self, field: &Element) -> DbResult<Element> {
        if !self.fields.contains(field) {
            return Err(DbError::FieldNotExists(field.to_string()));
        }
        let field = match field {
            Element::Spec(_, field) => &Element::raw(field),
            e => e,
        };
        if let Some(field) = self.mapping.field(field) {
            Ok(field.clone())
        } else {
            Ok(field.clone())
        }
    }
}

impl Scan for ProjectScan {
    fn before_first(&self) -> DbResult<()> {
        self.scan.before_first()
    }

    fn next(&self) -> DbResult<bool> {
        self.scan.next()
    }

    fn get_i32(&self, field: &Element) -> DbResult<i32> {
        let field = self.get_field(field)?;
        self.scan.get_i32(&field)
    }

    fn get_string(&self, field: &Element) -> DbResult<String> {
        let field = self.get_field(field)?;
        self.scan.get_string(&field)
    }

    fn get_val(&self, field: &Element) -> DbResult<crate::value::Value> {
        let field = self.get_field(field)?;
        self.scan.get_val(&field)
    }

    fn has_field(&self, field: &Element) -> DbResult<bool> {
        Ok(self.fields.contains(field))
    }

    fn close(&self) -> DbResult<()> {
        self.scan.close()
    }

    fn schema(&self) -> DbResult<Schema> {
        let schema = self.scan.schema()?;
        let mut project = SchemaBuilder::new(schema.table().clone());
        for (field, info) in schema.fields() {
            if self.fields.contains(&field) {
                project = project.add_field(field, info);
            }
        }
        Ok(project.build())
    }
}
