use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebtCategory {
    id: Uuid,
    name: String,
}

impl DebtCategory {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_uppercase(),
        }
    }
}

getters! {
    DebtCategory {
        id: Uuid,
        name: String,
    }
}

from_row_constructor! {
    DebtCategory {
        id: Uuid,
        name: String,
    }
}
