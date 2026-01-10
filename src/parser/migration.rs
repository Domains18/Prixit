use crate::error::{AnalyzeError, Result};
use crate::models::{Migration, SqlOperation};
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use std::fmt::format;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct MigrationDiscovery;

impl MigrationDiscovery {
    pub fn discover(migration_path: &str) -> Result<Vec<Migration>> {
        let path = Path::new(migration_path);

        if !path.exists() {
            return Err(AnalyzeError::MigrationDirError(format!(
                "Directort does not exist: {}",
                migration_path
            )));
        }

        let mut migrations = Vec::new();

        for entry in WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().id_dir() {
                if let Some(migration) = Self::parse_migration_dir(entry.path())? {
                    migrations.push(migration);
                }
            }
        }

        migrations.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(migrations)
    }

    fn parse_migration_dir(dir_path: &Path) -> Result<Option<Migration>> {
        let dir_name = dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AnalyzeError::InvalidMigration("invalid directory name".to_string()))?;

        let (timestamp, name) = Self::parse_migration_name(dir_name)?;

        let sql_path = dir_path.join("migration.sql");
        if !sql_path.exists() {
            return Ok(None);
        }

        let sql_content = fs::read_to_string(&sql_path)?;

        let id = dir_name.to_string();

        Ok(Some(Migration {
            id,
            name,
            timestamp,
            sql_content,
            operations: vec![],
            affected_tables: vec![],
        }))
    }

    fn parse_migration_name(name: &str) -> Result<(DateTime<Utc>, String)> {
        let re = Regex::new(r"^(\d{14})_(.+)$").unwrap();

        if let Some(caps) = re.captures(name) {
            let timestamp_str = &caps[1];
            let migration_name = caps[2].replace('_', " ");

            let naive_dt = NaiveDateTime::parse_from_str(timestamp_str, "%Y%m%d%H%M%S")
                .map_err(|e| AnalyzeError::InvalidMigration(format!("invalid timestamp: {}", e)))?;

            let timestamp = DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc);

            Ok((timestamp, migration_name))
        } else {
            Err(AnalyzeError::InvalidMigration(format!(
                "invalid migration name format: {}",
                name
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_migration_name() {
        let (timestamp, name) =
            MigrationDiscovery::parse_migration_name("20231201120000_init").unwrap();

        assert_eq!(name, "init");
        assert_eq!(
            timestamp.format("%Y%m%d%H%M%S").to_string(),
            "20231201120000"
        );
    }

    #[test]
    fn test_parse_migration_name_with_underscores() {
        let (_, name) =
            MigrationDiscovery::parse_migration_name("20231201120000_add_user_table").unwrap();

        assert_eq!(name, "add user table")
    }
}
