use chrono::{DateTime, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionStatus {
    #[default]
    Open,
    Closed,
}

impl From<String> for SessionStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "CLOSED" => SessionStatus::Closed,
            _ => SessionStatus::Open,
        }
    }
}

impl From<SessionStatus> for String {
    fn from(status: SessionStatus) -> Self {
        match status {
            SessionStatus::Open => "OPEN".to_string(),
            SessionStatus::Closed => "CLOSED".to_string(),
        }
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SessionStatus::Open => "OPEN",
            SessionStatus::Closed => "CLOSED",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    id: Uuid,
    session_date: NaiveDate,
    #[serde(default)]
    status: SessionStatus,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn new(session_date: NaiveDate) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_date,
            status: SessionStatus::Open,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn close(&mut self) -> HttpResult<()> {
        if self.status == SessionStatus::Closed {
            return Err(Box::new(HttpError::bad_request("Session already closed")));
        }

        self.status = SessionStatus::Closed;
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.status == SessionStatus::Open
    }
}

getters!(
    Session {
        id: Uuid,
        session_date: NaiveDate,
        status: SessionStatus,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
);

from_row_constructor! {
    Session {
        id: Uuid,
        session_date: NaiveDate,
        status: SessionStatus,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFilters {
    ids: Option<Vec<Uuid>>,
    statuses: Option<Vec<SessionStatus>>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

impl SessionFilters {
    pub fn with_ids(mut self, ids: Vec<Uuid>) -> Self {
        self.ids = Some(ids);
        self
    }

    pub fn with_optional_ids(mut self, ids: Option<Vec<Uuid>>) -> Self {
        if let Some(ids) = ids {
            self.ids = Some(ids);
        }
        self
    }

    pub fn with_statuses(mut self, statuses: Vec<SessionStatus>) -> Self {
        self.statuses = Some(statuses);
        self
    }

    pub fn with_optional_statuses(mut self, statuses: Option<Vec<SessionStatus>>) -> Self {
        if let Some(statuses) = statuses {
            self.statuses = Some(statuses);
        }
        self
    }

    pub fn with_start_date(mut self, start_date: NaiveDate) -> Self {
        self.start_date = Some(start_date);
        self
    }

    pub fn with_optional_start_date(mut self, start_date: Option<NaiveDate>) -> Self {
        if let Some(start_date) = start_date {
            self.start_date = Some(start_date);
        }
        self
    }

    pub fn with_end_date(mut self, end_date: NaiveDate) -> Self {
        self.end_date = Some(end_date);
        self
    }

    pub fn with_optional_end_date(mut self, end_date: Option<NaiveDate>) -> Self {
        if let Some(end_date) = end_date {
            self.end_date = Some(end_date);
        }
        self
    }
}

getters!(
    SessionFilters {
        ids: Option<Vec<Uuid>>,
        statuses: Option<Vec<SessionStatus>>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    }
);
