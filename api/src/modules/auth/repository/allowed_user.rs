use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::modules::auth::domain::allowed_user::AllowedUser;

#[async_trait]
pub trait AllowedUserRepository {
    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<AllowedUser>>;
    async fn get_by_email(&self, email: &str) -> HttpResult<Option<AllowedUser>>;
}

pub type DynAllowedUserRepository = dyn AllowedUserRepository + Send + Sync;

pub struct AllowedUserRepositoryImpl {
    pool: Pool<Postgres>,
}

impl AllowedUserRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl AllowedUserRepository for AllowedUserRepositoryImpl {
    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<AllowedUser>> {
        let row = sqlx::query(r#"SELECT * FROM auth.allowed_users WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| AllowedUser::from(entity::AllowedUserEntity::from(&r))))
    }

    async fn get_by_email(&self, email: &str) -> HttpResult<Option<AllowedUser>> {
        let row = sqlx::query(r#"SELECT * FROM auth.allowed_users WHERE email = $1"#)
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| AllowedUser::from(entity::AllowedUserEntity::from(&r))))
    }
}

pub mod entity {
    use chrono::NaiveDateTime;
    use sqlx::{postgres::PgRow, Row};
    use uuid::Uuid;

    use crate::modules::auth::domain::allowed_user::AllowedUser;

    pub struct AllowedUserEntity {
        pub id: Uuid,
        pub email: String,
        pub tenant_id: Uuid,
        pub role: String,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<&PgRow> for AllowedUserEntity {
        fn from(row: &PgRow) -> Self {
            Self {
                id: row.get("id"),
                email: row.get("email"),
                tenant_id: row.get("tenant_id"),
                role: row.get("role"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        }
    }

    impl From<AllowedUserEntity> for AllowedUser {
        fn from(entity: AllowedUserEntity) -> Self {
            AllowedUser::from_row(
                entity.id,
                entity.email,
                entity.tenant_id,
                entity.role.into(),
                entity.created_at.and_utc(),
                entity.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
