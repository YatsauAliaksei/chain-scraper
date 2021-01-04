use diesel::{Insertable, Queryable, Identifiable, AsChangeset};

use crate::db::schema::contracts;

pub const CREATE_SCRIPT: &str = "\
     CREATE TABLE IF NOT EXISTS contracts (\
         id SERIAL PRIMARY KEY,\
         address TEXT,\
         abi_json TEXT NOT NULL\
     )";

#[derive(Queryable, Identifiable, Debug, AsChangeset)]
// default 'if None - ignore field'
// #[changeset_options(treat_none_as_null="true")]
pub struct Contract {
    pub id: i32,
    pub address: Option<String>,
    pub abi_json: String,
}

#[derive(Insertable, Debug)]
#[table_name = "contracts"]
pub struct NewContract {
    pub address: Option<String>,
    pub abi_json: String,
}
