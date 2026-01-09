use thiserror::Error;

#[derive(Error. Debug)]
pub enum AnalyzeError {
    #[error("failed to read migration directory: {0}")]
    MigrationDirError(String),

    #[error("failed to parse SQL: {0}")]
    SqlParseError(String),

    #[error("failed to parse Prisma schema: {0}")]
    SchemaParseError(String),

    #[error["IO error: {0}"]]
    IoError(#[from] std::io::Error),

    #[error("invalid migration format: {0}")]
    InvalidMigration(String),
}

pub type Result<T> = std::result::Result<T, AnalyzeError>;
