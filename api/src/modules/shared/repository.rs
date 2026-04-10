use async_trait::async_trait;
use http_error::HttpResult;

#[async_trait]
pub trait Repository<T, Filters, EntityUuid> {
    /// List all items matching the filters.
    async fn list(&self, filters: &Filters) -> HttpResult<Vec<T>>;

    /// Get an item by its id.
    async fn get(&self, id: &EntityUuid) -> HttpResult<Option<T>>;

    /// Insert a new item.
    async fn insert(&self, item: T) -> HttpResult<T>;

    /// Insert multiple items.
    async fn insert_many(&self, items: Vec<T>) -> HttpResult<Vec<T>>;

    /// Update an item by its id.
    async fn update(&self, item: T) -> HttpResult<T>;

    /// Delete an item by its id.
    async fn delete(&self, id: &EntityUuid) -> HttpResult<()>;
}
