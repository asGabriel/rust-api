use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::modules::auth::domain::user::User;

#[async_trait]
pub trait UserRepository {
    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<User>>;
    async fn get_by_username(&self, username: &str) -> HttpResult<Option<User>>;
    async fn get_by_email(&self, email: &str) -> HttpResult<Option<User>>;
    async fn insert(&self, user: User) -> HttpResult<User>;
    async fn update(&self, user: User) -> HttpResult<()>;
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

    async fn get_by_username(&self, username: &str) -> HttpResult<Option<User>> {
        let row = sqlx::query(r#"SELECT * FROM auth.users WHERE username = $1"#)
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| User::from(entity::UserEntity::from(&r))))
    }

    async fn get_by_email(&self, email: &str) -> HttpResult<Option<User>> {
        let row = sqlx::query(r#"SELECT * FROM auth.users WHERE email = $1"#)
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| User::from(entity::UserEntity::from(&r))))
    }

    async fn insert(&self, user: User) -> HttpResult<User> {
        let entity = entity::UserEntity::from(user);

        let row = sqlx::query(
            r#"
            INSERT INTO auth.users (id, username, email, password_hash, name, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(entity.id)
        .bind(&entity.username)
        .bind(&entity.email)
        .bind(&entity.password_hash)
        .bind(&entity.name)
        .bind(entity.is_active)
        .bind(entity.created_at)
        .bind(entity.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(User::from(entity::UserEntity::from(&row)))
    }

    async fn update(&self, user: User) -> HttpResult<()> {
        let entity = entity::UserEntity::from(user);

        sqlx::query(
            r#"
            UPDATE auth.users SET 
                username = $2,
                email = $3,
                password_hash = $4,
                name = $5,
                is_active = $6,
                updated_at = $7
            WHERE id = $1
            "#,
        )
        .bind(entity.id)
        .bind(&entity.username)
        .bind(&entity.email)
        .bind(&entity.password_hash)
        .bind(&entity.name)
        .bind(entity.is_active)
        .bind(chrono::Utc::now().naive_utc())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

pub mod entity {
    use chrono::NaiveDateTime;
    use sqlx::{postgres::PgRow, Row};
    use uuid::Uuid;

    use crate::modules::auth::domain::user::User;

    pub struct UserEntity {
        pub id: Uuid,
        pub username: String,
        pub email: String,
        pub password_hash: String,
        pub name: String,
        pub is_active: bool,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<&PgRow> for UserEntity {
        fn from(row: &PgRow) -> Self {
            Self {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                name: row.get("name"),
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        }
    }

    impl From<User> for UserEntity {
        fn from(user: User) -> Self {
            Self {
                id: *user.id(),
                username: user.username().clone(),
                email: user.email().clone(),
                password_hash: user.password_hash().clone(),
                name: user.name().clone(),
                is_active: *user.is_active(),
                created_at: user.created_at().naive_utc(),
                updated_at: user.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<UserEntity> for User {
        fn from(entity: UserEntity) -> Self {
            User::from_row(
                entity.id,
                entity.username,
                entity.email,
                entity.password_hash,
                entity.name,
                entity.is_active,
                entity.created_at.and_utc(),
                entity.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
