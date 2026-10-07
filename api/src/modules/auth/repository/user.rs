use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::modules::auth::domain::user::User;

#[async_trait]
pub trait UserRepository {
    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<User>>;
    async fn get_by_google_sub(&self, google_sub: &str) -> HttpResult<Option<User>>;
    async fn insert(&self, user: User) -> HttpResult<User>;
    async fn update(&self, user: User) -> HttpResult<User>;
}

pub type DynUserRepository = dyn UserRepository + Send + Sync;

pub struct UserRepositoryImpl {
    pool: Pool<Postgres>,
}

impl UserRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<User>> {
        let row = sqlx::query(r#"SELECT * FROM auth.users WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| User::from(entity::UserEntity::from(&r))))
    }

    async fn get_by_google_sub(&self, google_sub: &str) -> HttpResult<Option<User>> {
        let row = sqlx::query(r#"SELECT * FROM auth.users WHERE google_sub = $1"#)
            .bind(google_sub)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| User::from(entity::UserEntity::from(&r))))
    }

    async fn insert(&self, user: User) -> HttpResult<User> {
        let entity = entity::UserEntity::from(user);

        let row = sqlx::query(
            r#"
            INSERT INTO auth.users (
                id, allowed_user_id, google_sub, email, name, avatar_url,
                created_at, updated_at, last_login_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(entity.id)
        .bind(entity.allowed_user_id)
        .bind(&entity.google_sub)
        .bind(&entity.email)
        .bind(&entity.name)
        .bind(&entity.avatar_url)
        .bind(entity.created_at)
        .bind(entity.updated_at)
        .bind(entity.last_login_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(User::from(entity::UserEntity::from(&row)))
    }

    async fn update(&self, user: User) -> HttpResult<User> {
        let entity = entity::UserEntity::from(user);

        let row = sqlx::query(
            r#"
            UPDATE auth.users SET
                email = $2,
                name = $3,
                avatar_url = $4,
                updated_at = $5,
                last_login_at = $6
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(entity.id)
        .bind(&entity.email)
        .bind(&entity.name)
        .bind(&entity.avatar_url)
        .bind(entity.updated_at)
        .bind(entity.last_login_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(User::from(entity::UserEntity::from(&row)))
    }
}

pub mod entity {
    use chrono::NaiveDateTime;
    use sqlx::{postgres::PgRow, Row};
    use uuid::Uuid;

    use crate::modules::auth::domain::user::User;

    pub struct UserEntity {
        pub id: Uuid,
        pub allowed_user_id: Uuid,
        pub google_sub: String,
        pub email: String,
        pub name: String,
        pub avatar_url: Option<String>,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
        pub last_login_at: NaiveDateTime,
    }

    impl From<&PgRow> for UserEntity {
        fn from(row: &PgRow) -> Self {
            Self {
                id: row.get("id"),
                allowed_user_id: row.get("allowed_user_id"),
                google_sub: row.get("google_sub"),
                email: row.get("email"),
                name: row.get("name"),
                avatar_url: row.get("avatar_url"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_login_at: row.get("last_login_at"),
            }
        }
    }

    impl From<User> for UserEntity {
        fn from(user: User) -> Self {
            Self {
                id: *user.id(),
                allowed_user_id: *user.allowed_user_id(),
                google_sub: user.google_sub().clone(),
                email: user.email().clone(),
                name: user.name().clone(),
                avatar_url: user.avatar_url().clone(),
                created_at: user.created_at().naive_utc(),
                updated_at: user.updated_at().map(|dt| dt.naive_utc()),
                last_login_at: user.last_login_at().naive_utc(),
            }
        }
    }

    impl From<UserEntity> for User {
        fn from(entity: UserEntity) -> Self {
            User::from_row(
                entity.id,
                entity.allowed_user_id,
                entity.google_sub,
                entity.email,
                entity.name,
                entity.avatar_url,
                entity.created_at.and_utc(),
                entity.updated_at.map(|dt| dt.and_utc()),
                entity.last_login_at.and_utc(),
            )
        }
    }
}
