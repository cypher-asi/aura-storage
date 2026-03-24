use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await?;

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Migrations complete");

    Ok(pool)
}
