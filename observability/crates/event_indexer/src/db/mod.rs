use diesel::{Connection, PgConnection};

pub mod models;
pub mod schema;

#[cold]
pub fn establish_connection(database_url: String) -> PgConnection {
    PgConnection::establish(&database_url)
        .unwrap_or_else(|err| panic!("Error connecting to {}: {:?}", database_url, err))
}
