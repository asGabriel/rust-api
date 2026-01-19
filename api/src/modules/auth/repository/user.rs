use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres, Row};
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

        Ok(row.map(|r| {
            User::from_row(
                r.get("id"),
                r.get("username"),
                r.get("email"),
                r.get("password_hash"),
                r.get("name"),
                r.get("is_active"),
                r.get::<chrono::NaiveDateTime, _>("created_at").and_utc(),
                r.get::<Option<chrono::NaiveDateTime>, _>("updated_at")
                    .map(|dt| dt.and_utc()),
            )
        }))
    }

    async fn get_by_username(&self, username: &str) -> HttpResult<Option<User>> {
        let row = sqlx::query(r#"SELECT * FROM auth.users WHERE username = $1"#)
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| {
            User::from_row(
                r.get("id"),
                r.get("username"),
                r.get("email"),
                r.get("password_hash"),
                r.get("name"),
                r.get("is_active"),
                r.get::<chrono::NaiveDateTime, _>("created_at").and_utc(),
                r.get::<Option<chrono::NaiveDateTime>, _>("updated_at")
                    .map(|dt| dt.and_utc()),
            )
        }))
    }

    async fn get_by_email(&self, email: &str) -> HttpResult<Option<User>> {
        let row = sqlx::query(r#"SELECT * FROM auth.users WHERE email = $1"#)
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| {
            User::from_row(
                r.get("id"),
                r.get("username"),
                r.get("email"),
                r.get("password_hash"),
                r.get("name"),
                r.get("is_active"),
                r.get::<chrono::NaiveDateTime, _>("created_at").and_utc(),
                r.get::<Option<chrono::NaiveDateTime>, _>("updated_at")
                    .map(|dt| dt.and_utc()),
            )
        }))
    }

    async fn insert(&self, user: User) -> HttpResult<User> {
        let row = sqlx::query(
            r#"
            INSERT INTO auth.users (id, username, email, password_hash, name, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, username, email, password_hash, name, is_active, created_at, updated_at
            "#,
        )
        .bind(user.id())
        .bind(user.username())
        .bind(user.email())
        .bind(user.password_hash())
        .bind(user.name())
        .bind(user.is_active())
        .bind(user.created_at().naive_utc())
        .bind(user.updated_at().map(|dt| dt.naive_utc()))
        .fetch_one(&self.pool)
        .await?;

        Ok(User::from_row(
            row.get("id"),
            row.get("username"),
            row.get("email"),
            row.get("password_hash"),
            row.get("name"),
            row.get("is_active"),
            row.get::<chrono::NaiveDateTime, _>("created_at").and_utc(),
            row.get::<Option<chrono::NaiveDateTime>, _>("updated_at").map(|dt| dt.and_utc()),
        ))
    }

    async fn update(&self, user: User) -> HttpResult<()> {
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
        .bind(user.id())
        .bind(user.username())
        .bind(user.email())
        .bind(user.password_hash())
        .bind(user.name())
        .bind(user.is_active())
        .bind(chrono::Utc::now().naive_utc())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
