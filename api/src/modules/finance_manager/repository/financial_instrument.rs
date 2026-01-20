use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::financial_instrument::FinancialInstrument,
    handler::financial_instrument::use_cases::FinancialInstrumentListFilters,
    repository::financial_instrument::entity::FinancialInstrumentEntity,
};

#[async_trait]
pub trait FinancialInstrumentRepository {
    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<FinancialInstrument>>;

    async fn get_by_identification(
        &self,
        identification: &str,
    ) -> HttpResult<Option<FinancialInstrument>>;

    async fn list(
        &self,
        filters: FinancialInstrumentListFilters,
    ) -> HttpResult<Vec<FinancialInstrument>>;

    async fn insert(&self, instrument: FinancialInstrument) -> HttpResult<FinancialInstrument>;

    async fn update(&self, instrument: FinancialInstrument) -> HttpResult<()>;
}

pub type DynFinancialInstrumentRepository = dyn FinancialInstrumentRepository + Send + Sync;

pub struct FinancialInstrumentRepositoryImpl {
    pool: Pool<Postgres>,
}

impl FinancialInstrumentRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl FinancialInstrumentRepository for FinancialInstrumentRepositoryImpl {
    async fn update(&self, instrument: FinancialInstrument) -> HttpResult<()> {
        let payload = FinancialInstrumentEntity::from(instrument);

        sqlx::query(
            r#"
            UPDATE finance_manager.financial_instrument SET 
                name = $2,
                owner = $3,
                instrument_type = $4,
                configuration = $5,
                updated_at = $6
            WHERE id = $1"#,
        )
        .bind(payload.id)
        .bind(payload.name)
        .bind(payload.owner)
        .bind(&payload.instrument_type)
        .bind(serde_json::to_value(payload.configuration).unwrap())
        .bind(payload.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_by_identification(
        &self,
        identification: &str,
    ) -> HttpResult<Option<FinancialInstrument>> {
        let identification_num: i32 = identification.parse().map_err(|_| {
            http_error::HttpError::bad_request(format!(
                "Invalid identification format: {}",
                identification
            ))
        })?;

        let row = sqlx::query(
            r#"SELECT * FROM finance_manager.financial_instrument WHERE identification = $1"#,
        )
        .bind(identification_num)
        .fetch_optional(&self.pool)
        .await?;

        let result = row.map(|r| FinancialInstrumentEntity {
            id: r.get("id"),
            client_id: r.get("client_id"),
            name: r.get("name"),
            owner: r.get("owner"),
            identification: r.get::<i32, _>("identification").to_string(),
            instrument_type: r.get::<String, _>("instrument_type"),
            configuration: serde_json::from_value(r.get("configuration")).unwrap(),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        });

        Ok(result.map(FinancialInstrument::from))
    }

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<FinancialInstrument>> {
        let row =
            sqlx::query(r#"SELECT * FROM finance_manager.financial_instrument WHERE id = $1"#)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        let result = row.map(|r| FinancialInstrumentEntity {
            id: r.get("id"),
            client_id: r.get("client_id"),
            name: r.get("name"),
            owner: r.get("owner"),
            identification: r.get::<i32, _>("identification").to_string(),
            instrument_type: r.get::<String, _>("instrument_type"),
            configuration: serde_json::from_value(r.get("configuration")).unwrap(),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        });

        Ok(result.map(FinancialInstrument::from))
    }

    async fn list(
        &self,
        filters: FinancialInstrumentListFilters,
    ) -> HttpResult<Vec<FinancialInstrument>> {
        let mut builder =
            QueryBuilder::new("SELECT * FROM finance_manager.financial_instrument WHERE 1=1");

        if let Some(client_id) = filters.client_id {
            builder.push(" AND client_id = ");
            builder.push_bind(client_id);
        }

        if let Some(ids) = filters.ids {
            builder.push(" AND id = ANY(");
            builder.push_bind(ids);
            builder.push(")");
        }

        if let Some(identifications) = filters.identifications {
            let identifications: Vec<i32> = identifications
                .iter()
                .map(|i| i.parse::<i32>().unwrap())
                .collect();
            builder.push(" AND identification = ANY(");
            builder.push_bind(identifications);
            builder.push(")");
        }

        if let Some(instrument_types) = filters.instrument_types {
            let types_as_str: Vec<String> = instrument_types
                .iter()
                .map(|t| t.as_str().to_string())
                .collect();
            builder.push(" AND instrument_type = ANY(");
            builder.push_bind(types_as_str);
            builder.push(")");
        }

        let query = builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        let rows: Vec<FinancialInstrumentEntity> = rows
            .into_iter()
            .map(|r| FinancialInstrumentEntity {
                id: r.get("id"),
                client_id: r.get("client_id"),
                name: r.get("name"),
                owner: r.get("owner"),
                identification: r.get::<i32, _>("identification").to_string(),
                instrument_type: r.get::<String, _>("instrument_type"),
                configuration: serde_json::from_value(r.get("configuration")).unwrap(),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect();

        Ok(rows.into_iter().map(FinancialInstrument::from).collect())
    }

    async fn insert(&self, instrument: FinancialInstrument) -> HttpResult<FinancialInstrument> {
        let payload = FinancialInstrumentEntity::from(instrument);

        let row = sqlx::query(
            r#"
            INSERT INTO finance_manager.financial_instrument (id, client_id, name, owner, instrument_type, configuration, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
        "#,
        )
        .bind(payload.id)
        .bind(payload.client_id)
        .bind(payload.name)
        .bind(payload.owner)
        .bind(&payload.instrument_type)
        .bind(serde_json::to_value(payload.configuration).unwrap())
        .bind(payload.created_at)
        .bind(payload.updated_at)
        .fetch_one(&self.pool)
        .await?;

        let result = FinancialInstrumentEntity {
            id: row.get("id"),
            client_id: row.get("client_id"),
            name: row.get("name"),
            owner: row.get("owner"),
            identification: row.get::<i32, _>("identification").to_string(),
            instrument_type: row.get::<String, _>("instrument_type"),
            configuration: serde_json::from_value(row.get("configuration")).unwrap(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        Ok(FinancialInstrument::from(result))
    }
}

pub mod entity {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::financial_instrument::{
        configuration::InstrumentConfiguration, FinancialInstrument, FinancialInstrumentType,
    };

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FinancialInstrumentEntity {
        pub id: Uuid,
        pub client_id: Uuid,
        pub name: String,
        pub owner: String,
        pub identification: String,
        pub instrument_type: String,
        pub configuration: InstrumentConfiguration,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<FinancialInstrument> for FinancialInstrumentEntity {
        fn from(instrument: FinancialInstrument) -> Self {
            FinancialInstrumentEntity {
                id: *instrument.id(),
                client_id: *instrument.client_id(),
                name: instrument.name().to_string(),
                owner: instrument.owner().to_string(),
                identification: instrument.identification().to_string(),
                instrument_type: instrument.instrument_type().as_str().to_string(),
                configuration: instrument.configuration().clone(),
                created_at: instrument.created_at().naive_utc(),
                updated_at: instrument.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<FinancialInstrumentEntity> for FinancialInstrument {
        fn from(dto: FinancialInstrumentEntity) -> Self {
            FinancialInstrument::from_row(
                dto.id,
                dto.client_id,
                dto.name,
                dto.owner,
                dto.identification,
                FinancialInstrumentType::from_str(&dto.instrument_type),
                dto.configuration,
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
