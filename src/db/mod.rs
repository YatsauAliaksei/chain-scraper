use std::env;

use diesel::{Connection, PgConnection};

mod model;
pub mod schema;

pub struct DBConnection {
    connection: PgConnection,
    url: String,
}

impl DBConnection {

    pub fn new(url: &str) -> Self {
        let connection = establish_connection(url);
        DBConnection {
            url: url.to_string(),
            connection,
        }
    }
}

fn establish_connection(database_url: &str) -> PgConnection {
    PgConnection::establish(database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

#[cfg(test)]
mod tests {
    use diesel::{Connection, PgConnection};
    use diesel::connection::SimpleConnection;
    use diesel::prelude::*;

    use model::contract::Contract;

    use crate::db::model::contract::NewContract;
    use crate::db::schema::contracts::dsl::*;

    use super::*;

    #[test]
    fn connection_db() {
        dotenv::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");

        let con = establish_connection(&database_url);

        db_init(&con);

        con.begin_test_transaction();

        let result: Contract = diesel::insert_into(contracts)
            .values(NewContract {
                address: None,
                abi_json: "hello".to_string(),
            })
            .get_result(&con)
            .expect("Error saving contract");


        println!("{:?}", result);
    }

    fn db_init(con: &PgConnection) {
        con.batch_execute(model::contract::CREATE_SCRIPT).expect("contracts table creation failed");
    }
}