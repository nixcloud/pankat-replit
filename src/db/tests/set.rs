use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use crate::db::tests::establish_connection;

#[test]
fn test_db_set() {
    let mut conn: SqliteConnection = establish_connection();
    assert!(true);
}
