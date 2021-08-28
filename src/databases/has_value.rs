pub trait HasValue: sqlx::Database {
    type Value;
}
