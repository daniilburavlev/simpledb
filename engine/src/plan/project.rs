use std::{collections::HashSet, rc::Rc};

use crate::schema::SchemaBuilder;
use crate::schema_mapping::SchemaMapping;
use crate::{
    element::Element,
    plan::Plan,
    scan::{Scan, project::ProjectScan},
    schema::Schema,
};
use common::DbResult;

pub(crate) struct ProjectPlan {
    plan: Rc<dyn Plan>,
    schema: Schema,
    mapping: SchemaMapping,
}

impl ProjectPlan {
    pub(crate) fn new(
        plan: Rc<dyn Plan>,
        fields: Vec<Element>,
        mapping: SchemaMapping,
    ) -> DbResult<Self> {
        let mut schema = SchemaBuilder::new(plan.schema()?.table().clone());
        let other = plan.schema()?;
        for field in fields {
            let source_field = match &field {
                Element::Spec(table, field) => {
                    let source_table = Element::raw(table);
                    let table = if let Some(Element::Raw(table)) = mapping.table(&source_table) {
                        table
                    } else {
                        table
                    };
                    Element::Spec(table.to_string(), field.to_string())
                }
                field => {
                    if let Some(field) = mapping.field(field) {
                        field.clone()
                    } else {
                        field.clone()
                    }
                }
            };
            if let Some(info) = other.info(&source_field) {
                schema = schema.add_field(field, info);
            }
        }
        Ok(Self {
            plan,
            schema: schema.build(),
            mapping,
        })
    }
}

impl Plan for ProjectPlan {
    fn open(&self) -> DbResult<Rc<dyn Scan>> {
        let scan = self.plan.open()?;
        let fields: HashSet<Element> = self.schema.fields().into_iter().map(|(f, _)| f).collect();
        Ok(Rc::new(ProjectScan::new(
            scan,
            fields,
            self.mapping.clone(),
        )))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        self.plan.blocks_accessed()
    }

    fn records_output(&self) -> DbResult<i32> {
        self.plan.records_output()
    }

    fn distinct_values(&self, field_name: &Element) -> DbResult<i32> {
        self.plan.distinct_values(field_name)
    }

    fn schema(&self) -> DbResult<Schema> {
        Ok(self.schema.clone())
    }
}
