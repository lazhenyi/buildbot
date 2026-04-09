//! Database initializer

use sea_orm::{Database as SeaDatabase, DatabaseConnection, DbErr};

/// Initialize the database connection
pub async fn init_database(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    let db = SeaDatabase::connect(database_url).await?;
    Ok(db)
}
