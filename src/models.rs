use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub done: bool,
    pub detail: String,
    pub priority: i64,
    #[allow(dead_code)]
    pub created_at: i64,
    #[allow(dead_code)]
    pub updated_at: i64,
}

impl Task {
    pub fn new(title: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Self {
            id: 0,
            title: title.to_string(),
            done: false,
            detail: String::new(),
            priority: 0,
            created_at: now,
            updated_at: now,
        }
    }
}
