use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::finance::{
    domain::list::List,
    handler::list::use_cases::{CreateListRequest, UpdateListRequest},
    repository::list::DynListRepository,
};
use std::sync::Arc;

pub type DynListHandler = dyn ListHandler + Send + Sync;

#[async_trait]
pub trait ListHandler {
    async fn register_new_list(
        &self,
        client_id: Uuid,
        request: CreateListRequest,
    ) -> HttpResult<List>;

    async fn list_lists(&self, client_id: Uuid) -> HttpResult<Vec<List>>;

    async fn update_list(
        &self,
        client_id: Uuid,
        list_id: Uuid,
        request: UpdateListRequest,
    ) -> HttpResult<List>;

    async fn delete_list(&self, client_id: Uuid, list_id: Uuid) -> HttpResult<()>;
}

#[derive(Clone)]
pub struct ListHandlerImpl {
    pub list_repository: Arc<DynListRepository>,
}

#[async_trait]
impl ListHandler for ListHandlerImpl {
    async fn register_new_list(
        &self,
        client_id: Uuid,
        request: CreateListRequest,
    ) -> HttpResult<List> {
        let list = List::new(client_id, request.name);
        self.list_repository.insert(list).await
    }

    async fn list_lists(&self, client_id: Uuid) -> HttpResult<Vec<List>> {
        self.list_repository.list(client_id).await
    }

    async fn update_list(
        &self,
        client_id: Uuid,
        list_id: Uuid,
        request: UpdateListRequest,
    ) -> HttpResult<List> {
        let mut list = self
            .list_repository
            .get_by_id(&list_id)
            .await?
            .or_not_found("list", list_id.to_string())?;

        if list.client_id() != &client_id {
            return Err(Box::new(HttpError::forbidden(
                "You don't have permission to update this list",
            )));
        }

        if let Some(name) = request.name {
            list.set_name(name);
        }

        self.list_repository.update(list).await
    }

    async fn delete_list(&self, client_id: Uuid, list_id: Uuid) -> HttpResult<()> {
        let list = self
            .list_repository
            .get_by_id(&list_id)
            .await?
            .or_not_found("list", list_id.to_string())?;

        if list.client_id() != &client_id {
            return Err(Box::new(HttpError::forbidden(
                "You don't have permission to delete this list",
            )));
        }

        self.list_repository.delete(list_id).await
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateListRequest {
        pub name: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateListRequest {
        pub name: Option<String>,
    }
}
