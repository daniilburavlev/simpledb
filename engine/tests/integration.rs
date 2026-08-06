#[cfg(test)]
mod tests {
    use common::error::DbError;
    use engine::{SimpleDB, element::Element};
    use tempfile::tempdir;

    #[test]
    fn insert_and_select_with_index() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();

        db.execute(&tx, "CREATE TABLE users(id INT, name VARCHAR(16))")
            .unwrap();
        tx.commit().unwrap();

        db.execute(&tx, "CREATE INDEX users_ids ON users(id)")
            .unwrap();
        tx.commit().unwrap();

        db.execute(&tx, "INSERT INTO users(id, name) VALUES(1, 'Alice')")
            .unwrap();
        db.execute(&tx, "INSERT INTO users(id, name) VALUES(2, 'Bob')")
            .unwrap();
        tx.commit().unwrap();

        let result = db
            .query(&tx, "SELECT id, name FROM users WHERE id = 2")
            .unwrap();
        assert!(result.next().unwrap());
        assert_eq!(result.get_i32(&Element::raw("id")).unwrap(), 2);
        assert_eq!(result.get_string(&Element::raw("name")).unwrap(), "Bob");
        tx.commit().unwrap();
    }

    #[test]
    #[ignore]
    fn insert_unknown_name() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();

        db.execute(&tx, "CREATE TABLE users(id INT)").unwrap();
        tx.commit().unwrap();

        if !matches!(
            db.execute(&tx, "INSERT INTO users(name) VALUES('Alice')"),
            Err(DbError::FieldNotExists(_))
        ) {
            panic!("insert not validate fields");
        };
        tx.commit().unwrap();
    }

    #[test]
    #[ignore]
    fn select_unknown_name() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();

        db.execute(&tx, "CREATE TABLE users(id INT)").unwrap();
        tx.commit().unwrap();

        if !matches!(
            db.query(&tx, "SELECT name FROM users"),
            Err(DbError::FieldNotExists(_))
        ) {
            panic!("select not validate fields");
        }
        tx.commit().unwrap();
    }

    #[test]
    fn fill_index_with_data_existed_before() {
        let dir = tempdir().unwrap();

        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        db.execute(&tx, "CREATE TABLE users(id INT)").unwrap();
        db.execute(&tx, "INSERT INTO users(id) VALUES(1)").unwrap();
        db.execute(&tx, "CREATE INDEX users_ids ON users(id)").unwrap();
        let result = db.query(&tx, "SELECT id FROM users WHERE id = 1").unwrap();
        assert!(result.next().unwrap());
        assert_eq!(result.get_i32(&Element::raw("id")).unwrap(), 1);
    }

    #[test]
    fn create_index_on_unexisted_table() {
        let dir = tempdir().unwrap();

        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        let result = db.execute(&tx, "CREATE INDEX invalid ON unexisted(id)");
        assert!(matches!(
            result.err().unwrap(),
            DbError::RelationNotExists(s) if s == "unexisted"
        ));
    }

    #[test]
    fn create_index_on_unexisted_field() {
        let dir = tempdir().unwrap();

        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        db.execute(&tx, "CREATE TABLE existed(id INT)").unwrap();
        tx.commit().unwrap();
        let result = db.execute(&tx, "CREATE INDEX invalid ON existed(value)");
        let err = result.err().unwrap();
        assert!(matches!(err, DbError::FieldNotExists(s) if s == "value"));
    }
}
