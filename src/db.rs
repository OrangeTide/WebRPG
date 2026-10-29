use diesel::SqliteConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use once_cell::sync::OnceCell;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

static POOL: OnceCell<DbPool> = OnceCell::new();

/// Migrations compiled into the binary from the `migrations/` directory, so a
/// fresh database (such as a new container volume) can be brought up to schema
/// without a separate `diesel migration run`.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn init_pool(database_url: &str) -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = Pool::builder()
        .max_size(10)
        .build(manager)
        .expect("Failed to create database pool");
    POOL.set(pool.clone())
        .expect("Database pool already initialized");
    pool
}

pub fn get_pool() -> &'static DbPool {
    POOL.get().expect("Database pool not initialized")
}

pub fn get_conn() -> PooledConnection<ConnectionManager<SqliteConnection>> {
    get_pool()
        .get()
        .expect("Failed to get connection from pool")
}

/// Apply any pending migrations against the pool's database. Runs at startup.
pub fn run_migrations() {
    let mut conn = get_conn();
    let applied = conn
        .run_pending_migrations(MIGRATIONS)
        .expect("Failed to run database migrations");
    if !applied.is_empty() {
        println!("Applied {} database migration(s)", applied.len());
    }
}
