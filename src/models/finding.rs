use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub severity: Severity,
    pub category: Category,
    pub migration_id: Option<String>,
    pub table: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Category {
    Perfomance,
    Safety,
    BestPractice,
    DataIntegrity,
}

impl Finding {
    pub fn new(severity: Severity, category: Category, message: impl Into<String>) -> Self {
        Self {
            severity,
            category,
            migration_id: None,
            table: None,
            message: message.into(),
            suggestion: None,
        }
    }
}

pub fn with_migration(mut self, migration_id: impl Into<String>) -> Self {
    self.migration_id = Some(migration_id.into());
    self
}

pub fn with_table(mut self, table: impl Into<String>) -> Self {
    self.table = Some(table.into());
    self
}

pub fn with_migration(mut self, suggestion: impl Into<String>) -> Self {
    self.suggestion = Some(suggestion.into());
    self
}
