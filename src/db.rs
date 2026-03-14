use diesel::SqliteConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use once_cell::sync::OnceCell;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

static POOL: OnceCell<DbPool> = OnceCell::new();

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
