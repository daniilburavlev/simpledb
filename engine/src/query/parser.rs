use common::{DbResult, error::DbError};

use crate::query::command::insert::ParsedInsertQuery;
use crate::query::command::select::ParsedSelectQuery;
use crate::schema::SchemaBuilder;
use crate::{
    element::Element,
    predicate::{Expression, Predicate, Term},
    query::{
        command::{Command, DeleteData, GroupByData, IndexData, TableData, UpdateData, ViewData},
        lexer::Lexer,
        token::Token,
    },
    schema::Schema,
    sort_by::SortByData,
    value::Value,
};

pub(crate) struct Parser {
    lexer: Lexer,
}

impl Parser {
    pub(crate) fn new(s: &str) -> DbResult<Self> {
        Ok(Self {
            lexer: Lexer::new(s)?,
        })
    }

    fn element(&self) -> DbResult<Element> {
        let id = self.lexer.eat_id()?;
        if self.lexer.match_delim('.') {
            self.lexer.eat_delimiter('.')?;
            let spec = self.lexer.eat_id()?;
            Ok(Element::Spec(id, spec))
        } else if self.lexer.match_id() {
            let name = self.lexer.eat_id()?;
            Ok(Element::View(id, name))
        } else {
            Ok(Element::Raw(id))
        }
    }

    fn field(&self) -> DbResult<String> {
        self.lexer.eat_id()
    }

    fn constant(&self) -> DbResult<Value> {
        if self.lexer.match_string_constant() {
            self.lexer.eat_string_constant()
        } else {
            self.lexer.eat_int_constant()
        }
    }

    fn expression(&self) -> DbResult<Expression> {
        if self.lexer.match_id() {
            Ok(Expression::Field(self.element()?))
        } else {
            Ok(Expression::Value(self.constant()?))
        }
    }

    fn term(&self) -> DbResult<Term> {
        let left = self.expression()?;
        self.lexer.eat_delimiter('=')?;
        let right = self.expression()?;
        Ok(Term::new(left, right))
    }

    fn predicate(&self) -> DbResult<Predicate> {
        let pred = Predicate::new(self.term()?);
        if self.lexer.match_keyword(Token::And) {
            self.lexer.eat_keyword(Token::And)?;
            pred.conjoin_with(&self.predicate()?)?;
        }
        Ok(pred)
    }

    pub(crate) fn query(&self) -> DbResult<Command> {
        self.lexer.eat_keyword(Token::Select)?;
        let fields = self.select_list()?;

        self.lexer.eat_keyword(Token::From)?;
        let (tables, predicate) = self.claude_from()?;

        if self.lexer.match_keyword(Token::Where) {
            self.lexer.eat_keyword(Token::Where)?;
            predicate.conjoin_with(&self.predicate()?)?;
        }

        let table = if tables.len() == 1 {
            tables.into_iter().next().unwrap()
        } else {
            Element::array(tables.into_iter().collect())
        };

        let mut group_by = GroupByData::default();
        if self.lexer.match_keyword(Token::Group) {
            self.lexer.eat_keyword(Token::Group)?;
            self.lexer.eat_keyword(Token::By)?;
            group_by = self.group_by()?;
        }

        let mut order_by = SortByData::default();
        if self.lexer.match_keyword(Token::Sort) {
            self.lexer.eat_keyword(Token::Sort)?;
            self.lexer.eat_keyword(Token::By)?;
            order_by = self.order_by()?;
        }
        self.check_remainder()?;
        Ok(Command::Query(ParsedSelectQuery {
            fields,
            table,
            predicate,
            group_by,
            order_by,
        }))
    }

    fn claude_from(&self) -> DbResult<(Vec<Element>, Predicate)> {
        let mut tables = vec![self.element()?];
        let predicate = Predicate::default();
        loop {
            if self.lexer.match_delim(',') {
                self.lexer.eat_delimiter(',')?;
                tables.push(self.element()?);
            } else if self.lexer.match_keyword(Token::Join) {
                self.lexer.eat_keyword(Token::Join)?;
                tables.push(self.element()?);
                self.lexer.eat_keyword(Token::On)?;
                predicate.conjoin_with(&self.predicate()?)?;
            } else {
                break;
            }
        }
        Ok((tables, predicate))
    }

    fn select_list(&self) -> DbResult<Vec<Element>> {
        let mut fields = vec![self.element()?];
        while self.lexer.match_delim(',') {
            self.lexer.eat_delimiter(',')?;
            fields.push(self.element()?);
        }
        Ok(fields)
    }

    pub fn is_update_cmd(&self) -> DbResult<Command> {
        if self.lexer.match_keyword(Token::Insert) {
            self.insert()
        } else if self.lexer.match_keyword(Token::Delete) {
            self.delete()
        } else if self.lexer.match_keyword(Token::Update) {
            self.update()
        } else {
            self.create()
        }
    }

    fn create(&self) -> DbResult<Command> {
        self.lexer.eat_keyword(Token::Create)?;
        if self.lexer.match_keyword(Token::Table) {
            self.create_table()
        } else if self.lexer.match_keyword(Token::View) {
            self.create_view()
        } else {
            self.create_index()
        }
    }

    fn update(&self) -> DbResult<Command> {
        self.lexer.eat_keyword(Token::Update)?;
        let table = self.lexer.eat_id()?;
        self.lexer.eat_keyword(Token::Set)?;
        let field = self.element()?;
        self.lexer.eat_delimiter('=')?;
        let value = self.expression()?;
        let mut predicate = Predicate::default();
        if self.lexer.match_keyword(Token::Where) {
            self.lexer.eat_keyword(Token::Where)?;
            predicate = self.predicate()?;
        }
        Ok(Command::Update(UpdateData {
            table,
            field,
            value,
            predicate,
        }))
    }

    fn delete(&self) -> DbResult<Command> {
        self.lexer.eat_keyword(Token::Delete)?;
        self.lexer.eat_keyword(Token::From)?;
        let name = self.lexer.eat_id()?;
        let mut predicate = Predicate::default();
        if self.lexer.match_keyword(Token::Where) {
            self.lexer.eat_keyword(Token::Where)?;
            predicate = self.predicate()?;
        }
        Ok(Command::Delete(DeleteData { name, predicate }))
    }

    fn insert(&self) -> DbResult<Command> {
        self.lexer.eat_keyword(Token::Insert)?;
        self.lexer.eat_keyword(Token::Into)?;

        let table = self.element()?;

        self.lexer.eat_delimiter('(')?;
        let fields = self.field_list()?;
        self.lexer.eat_delimiter(')')?;

        self.lexer.eat_keyword(Token::Values)?;

        let values = self.insert_values()?;

        Ok(Command::Insert(ParsedInsertQuery {
            table,
            fields,
            values,
        }))
    }

    fn insert_values(&self) -> DbResult<Vec<Vec<Value>>> {
        let mut all = vec![];
        loop {
            self.lexer.eat_delimiter('(')?;
            let values = self.values_list()?;
            all.push(values);
            self.lexer.eat_delimiter(')')?;
            if self.lexer.match_delim(',') {
                self.lexer.eat_delimiter(',')?;
            } else {
                break;
            }
        }
        Ok(all)
    }

    fn field_list(&self) -> DbResult<Vec<Element>> {
        let mut fields = vec![];
        fields.push(self.element()?);
        while self.lexer.match_delim(',') {
            self.lexer.eat_delimiter(',')?;
            fields.push(self.element()?);
        }
        Ok(fields)
    }

    fn values_list(&self) -> DbResult<Vec<Value>> {
        let mut constants = vec![];
        constants.push(self.constant()?);
        while self.lexer.match_delim(',') {
            self.lexer.eat_delimiter(',')?;
            constants.push(self.constant()?);
        }
        Ok(constants)
    }

    fn create_table(&self) -> DbResult<Command> {
        self.lexer.eat_keyword(Token::Table)?;
        let name = self.lexer.eat_id()?;
        self.lexer.eat_delimiter('(')?;
        let schema = self.field_definitions(&name)?;
        self.lexer.eat_delimiter(')')?;
        Ok(Command::CreateTable(TableData { name, schema }))
    }

    fn field_definitions(&self, table: &str) -> DbResult<Schema> {
        let mut schema = SchemaBuilder::new(Element::raw(table));
        schema = self.field_definition(schema)?;
        while self.lexer.match_delim(',') {
            self.lexer.eat_delimiter(',')?;
            schema = self.field_definition(schema)?;
        }
        Ok(schema.build())
    }

    fn field_definition(&self, schema: SchemaBuilder) -> DbResult<SchemaBuilder> {
        let field_name = self.element()?;
        self.field_type(field_name, schema)
    }

    fn field_type(
        &self,
        field_name: Element,
        mut schema: SchemaBuilder,
    ) -> DbResult<SchemaBuilder> {
        if self.lexer.match_keyword(Token::Int) {
            self.lexer.eat_keyword(Token::Int)?;
            schema = schema.add_int_field(field_name);
        } else {
            self.lexer.eat_keyword(Token::Varchar)?;
            self.lexer.eat_delimiter('(')?;
            let str_len = self.lexer.eat_int_constant()?;
            self.lexer.eat_delimiter(')')?;
            match str_len {
                Value::Integer(value) => schema = schema.add_string_field(field_name, value),
                _ => return Err(DbError::BadSyntax),
            }
        }
        Ok(schema)
    }

    pub fn create_view(&self) -> DbResult<Command> {
        self.lexer.eat_keyword(Token::View)?;
        let name = self.lexer.eat_id()?;
        self.lexer.eat_keyword(Token::As)?;
        if let Command::Query(query) = self.query()? {
            Ok(Command::CreateView(ViewData { name, query }))
        } else {
            Err(DbError::BadSyntax)
        }
    }

    pub fn create_index(&self) -> DbResult<Command> {
        self.lexer.eat_keyword(Token::Index)?;
        let index = self.lexer.eat_id()?;
        self.lexer.eat_keyword(Token::On)?;
        let table = self.lexer.eat_id()?;
        self.lexer.eat_delimiter('(')?;
        let field = self.field()?;
        self.lexer.eat_delimiter(')')?;
        Ok(Command::CreateIndex(IndexData {
            index,
            table,
            field,
        }))
    }

    fn group_by(&self) -> DbResult<GroupByData> {
        let mut fields = vec![self.element()?];
        while self.lexer.match_delim(',') {
            self.lexer.eat_delimiter(',')?;
            fields.push(self.element()?);
        }
        Ok(GroupByData { fields })
    }

    fn order_by(&self) -> DbResult<SortByData> {
        let mut fields = vec![self.element()?];
        while self.lexer.match_delim(',') {
            self.lexer.eat_delimiter(',')?;
            fields.push(self.element()?);
        }
        Ok(SortByData { fields })
    }

    fn check_remainder(&self) -> DbResult<()> {
        if self.lexer.is_empty() {
            Ok(())
        } else {
            Err(DbError::UnexpectedToken(self.lexer.eat()?.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_table() {
        let query = "CREATE TABLE users(name VARCHAR(256), age INTEGER)";
        let parser = Parser::new(query).unwrap();
        let create = parser.create().unwrap();
        assert_eq!(query, create.to_string());
    }

    #[test]
    fn create_view() {
        let query = "CREATE VIEW users AS SELECT * FROM users";
        let parser = Parser::new(query).unwrap();
        let create = parser.create().unwrap();
        assert_eq!(query, create.to_string());
    }

    #[test]
    fn create_index() {
        let query = "CREATE INDEX user_names ON users(name)";
        let parser = Parser::new(query).unwrap();
        let create = parser.create().unwrap();
        assert_eq!(query, create.to_string());
    }

    #[test]
    fn insert() {
        let query = "INSERT INTO users(name, age) VALUES('User', 18)";
        let parser = Parser::new(query).unwrap();
        let insert = parser.insert().unwrap();
        assert_eq!(query, insert.to_string());
    }

    #[test]
    fn update() {
        let query = "UPDATE users SET name='User' WHERE age=18";
        let parser = Parser::new(query).unwrap();
        let update = parser.is_update_cmd().unwrap();
        assert_eq!(query, update.to_string());
    }

    #[test]
    fn delete() {
        let query = "DELETE FROM users WHERE age=18";
        let parser = Parser::new(query).unwrap();
        let insert = parser.is_update_cmd().unwrap();
        assert_eq!(query, insert.to_string());
    }

    #[test]
    fn select() {
        let query = "SELECT *, name, age FROM users";
        let parser = Parser::new(query).unwrap();
        let select = parser.query().unwrap();
        assert_eq!(query, select.to_string());
    }

    #[test]
    fn select_where() {
        let query = "SELECT * FROM users WHERE name='User User'";
        let parser = Parser::new(query).unwrap();
        let select = parser.query().unwrap();
        assert_eq!(query, select.to_string());
    }

    #[test]
    fn select_with_group() {
        let query = "SELECT * FROM users WHERE name='User' AND id='50' GROUP BY id";
        let parser = Parser::new(query).unwrap();
        let select = parser.query().unwrap();
        assert_eq!(query, select.to_string());
    }

    #[test]
    #[should_panic]
    fn select_invalid() {
        let query = "SELECT * FROM users WHERE GROUP BY id";
        let parser = Parser::new(query).unwrap();
        let select = parser.query().unwrap();
        assert_eq!(query, select.to_string());
    }

    #[test]
    fn select_with_views_and_specs() {
        let query = "SELECT id i, name n, age a FROM users u WHERE u.id=10";
        let parser = Parser::new(query).unwrap();
        let select = parser.query().unwrap();
        assert_eq!(query, select.to_string());
    }

    #[test]
    fn insert_many_values() {
        let query = "INSERT INTO users(id, name) VALUES(1, 'Alice'), (2, 'Bob')";
        let parser = Parser::new(query).unwrap();
        let Command::Insert(insert) = parser.insert().unwrap() else {
            panic!("cannot parse insert query");
        };
        assert_eq!(2, insert.values.len());
        assert_eq!(query, insert.to_string());
    }
}
