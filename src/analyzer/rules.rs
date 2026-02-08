use crate::models::{
    Finding, Severity, Category, Migration,
    SqlOperation, TableAlteration,
};
use crate::models::schema::SchemaState;

/// Non-nullable column added without DEFAULT on an existing table.
pub fn check_non_nullable_without_default(
    migration: &Migration,
    schema_before: &SchemaState,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for op in &migration.operations {
        if let SqlOperation::AlterTable { table_name, alterations } = op {
            if schema_before.table_exists(table_name) {
                for alt in alterations {
                    if let TableAlteration::AddColumn(col) = alt {
                        if !col.nullable && col.default.is_none() {
                            findings.push(
                                Finding::new(
                                    Severity::Error,
                                    Category::Safety,
                                    format!(
                                        "Non-nullable column '{}' added to existing table '{}' without a DEFAULT value. \
                                         This will fail if the table has existing rows.",
                                        col.name, table_name
                                    ),
                                )
                                .with_migration(&migration.id)
                                .with_table(table_name)
                                .with_suggestion(
                                    "Add a DEFAULT value, or make the column nullable first, \
                                     backfill data, then set NOT NULL."
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    findings
}

/// Destructive operations: DROP TABLE, DROP COLUMN.
pub fn check_destructive_operations(migration: &Migration) -> Vec<Finding> {
    let mut findings = Vec::new();

    for op in &migration.operations {
        match op {
            SqlOperation::DropTable { table_name } => {
                findings.push(
                    Finding::new(
                        Severity::Warning,
                        Category::Safety,
                        format!(
                            "Table '{}' is being dropped. This is irreversible and will lose all data.",
                            table_name
                        ),
                    )
                    .with_migration(&migration.id)
                    .with_table(table_name)
                    .with_suggestion(
                        "Ensure a backup exists or that this table's data is no longer needed."
                    ),
                );
            }

            SqlOperation::AlterTable { table_name, alterations } => {
                for alt in alterations {
                    if let TableAlteration::DropColumn(col_name) = alt {
                        findings.push(
                            Finding::new(
                                Severity::Warning,
                                Category::Safety,
                                format!(
                                    "Column '{}' is being dropped from table '{}'. This is irreversible.",
                                    col_name, table_name
                                ),
                            )
                            .with_migration(&migration.id)
                            .with_table(table_name)
                            .with_suggestion(
                                "Consider renaming the column first and deploying, to verify nothing depends on it."
                            ),
                        );
                    }
                }
            }

            _ => {}
        }
    }

    findings
}

/// Foreign key without a corresponding index (checked AFTER migration applied).
pub fn check_missing_fk_index(
    migration: &Migration,
    schema_after: &SchemaState,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for op in &migration.operations {
        if let SqlOperation::AlterTable { table_name, alterations } = op {
            for alt in alterations {
                if let TableAlteration::AddForeignKey { columns, references_table, .. } = alt {
                    if !schema_after.has_index_on(table_name, columns) {
                        findings.push(
                            Finding::new(
                                Severity::Warning,
                                Category::Perfomance,
                                format!(
                                    "Foreign key on '{}({})' referencing '{}' has no index. \
                                     JOINs and cascading deletes will be slow.",
                                    table_name,
                                    columns.join(", "),
                                    references_table,
                                ),
                            )
                            .with_migration(&migration.id)
                            .with_table(table_name)
                            .with_suggestion(
                                format!(
                                    "Add an index: CREATE INDEX ON {}({});",
                                    table_name, columns.join(", ")
                                )
                            ),
                        );
                    }
                }
            }
        }
    }

    findings
}

/// Type changes that could lose data.
pub fn check_type_changes(
    migration: &Migration,
    schema_before: &SchemaState,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for op in &migration.operations {
        if let SqlOperation::AlterTable { table_name, alterations } = op {
            if let Some(table) = schema_before.tables.get(table_name) {
                for alt in alterations {
                    if let TableAlteration::AlterColumn { column_name, new_type: Some(new_type), .. } = alt {
                        if let Some(col) = table.columns.get(column_name) {
                            if is_potentially_lossy_change(&col.data_type, new_type) {
                                findings.push(
                                    Finding::new(
                                        Severity::Warning,
                                        Category::DataIntegrity,
                                        format!(
                                            "Column '{}.{}' type changed from '{}' to '{}'. \
                                             This could cause data loss or conversion errors.",
                                            table_name, column_name, col.data_type, new_type,
                                        ),
                                    )
                                    .with_migration(&migration.id)
                                    .with_table(table_name)
                                    .with_suggestion(
                                        "Consider adding a new column, migrating data, then dropping the old column."
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    findings
}

/// Adding a unique constraint to an existing table could fail with duplicate data.
pub fn check_add_unique_constraint(
    migration: &Migration,
    schema_before: &SchemaState,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for op in &migration.operations {
        if let SqlOperation::CreateIndex { table_name, columns, unique: true, .. } = op {
            if schema_before.table_exists(table_name) {
                findings.push(
                    Finding::new(
                        Severity::Warning,
                        Category::Safety,
                        format!(
                            "Unique index on '{}({})' added to existing table. \
                             This will fail if duplicate values exist.",
                            table_name, columns.join(", "),
                        ),
                    )
                    .with_migration(&migration.id)
                    .with_table(table_name)
                    .with_suggestion(
                        "Verify no duplicate data exists before applying this migration."
                    ),
                );
            }
        }
    }

    findings
}

/// Nullable to NOT NULL conversion needs data backfill.
pub fn check_nullable_to_not_null(
    migration: &Migration,
    schema_before: &SchemaState,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for op in &migration.operations {
        if let SqlOperation::AlterTable { table_name, alterations } = op {
            if let Some(table) = schema_before.tables.get(table_name) {
                for alt in alterations {
                    if let TableAlteration::AlterColumn {
                        column_name,
                        new_nullable: Some(false),
                        ..
                    } = alt
                    {
                        if let Some(col) = table.columns.get(column_name) {
                            if col.nullable {
                                findings.push(
                                    Finding::new(
                                        Severity::Warning,
                                        Category::DataIntegrity,
                                        format!(
                                            "Column '{}.{}' changed from nullable to NOT NULL. \
                                             Existing NULL values will cause this to fail.",
                                            table_name, column_name,
                                        ),
                                    )
                                    .with_migration(&migration.id)
                                    .with_table(table_name)
                                    .with_suggestion(
                                        "Add a data migration step to backfill NULL values before setting NOT NULL."
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    findings
}

/// Summary of what each migration does.
pub fn migration_summary(migration: &Migration) -> Vec<Finding> {
    let mut tables_created = Vec::new();
    let mut tables_altered = Vec::new();
    let mut tables_dropped = Vec::new();
    let mut indexes_created = 0u32;

    for op in &migration.operations {
        match op {
            SqlOperation::CreateTable { table_name, columns } => {
                tables_created.push(format!("{} ({} columns)", table_name, columns.len()));
            }
            SqlOperation::AlterTable { table_name, alterations } => {
                tables_altered.push(format!("{} ({} changes)", table_name, alterations.len()));
            }
            SqlOperation::DropTable { table_name } => {
                tables_dropped.push(table_name.clone());
            }
            SqlOperation::CreateIndex { .. } => {
                indexes_created += 1;
            }
            _ => {}
        }
    }

    let mut parts = Vec::new();
    if !tables_created.is_empty() {
        parts.push(format!("Creates: {}", tables_created.join(", ")));
    }
    if !tables_altered.is_empty() {
        parts.push(format!("Alters: {}", tables_altered.join(", ")));
    }
    if !tables_dropped.is_empty() {
        parts.push(format!("Drops: {}", tables_dropped.join(", ")));
    }
    if indexes_created > 0 {
        parts.push(format!("{} index(es) created", indexes_created));
    }

    if parts.is_empty() {
        return Vec::new();
    }

    vec![
        Finding::new(
            Severity::Info,
            Category::BestPractice,
            format!("Migration '{}': {}", migration.name, parts.join(". ")),
        )
        .with_migration(&migration.id)
    ]
}

fn is_potentially_lossy_change(old_type: &str, new_type: &str) -> bool {
    let old_upper = old_type.to_uppercase();
    let new_upper = new_type.to_uppercase();

    // TEXT/VARCHAR to something non-string
    if (old_upper.contains("TEXT") || old_upper.contains("VARCHAR"))
        && !(new_upper.contains("TEXT") || new_upper.contains("VARCHAR"))
    {
        return true;
    }

    // Numeric precision reduction
    let numeric_rank = |t: &str| -> i32 {
        let t = t.to_uppercase();
        if t.contains("BIGINT") || t.contains("BIGSERIAL") { 4 }
        else if t.contains("SMALLINT") { 2 }
        else if t.contains("INT") || t.contains("SERIAL") { 3 }
        else if t.contains("DOUBLE") || t.contains("FLOAT8") { 4 }
        else if t.contains("REAL") || t.contains("FLOAT4") || t.contains("FLOAT") { 3 }
        else { 0 }
    };

    let old_rank = numeric_rank(&old_upper);
    let new_rank = numeric_rank(&new_upper);
    if old_rank > 0 && new_rank > 0 && new_rank < old_rank {
        return true;
    }

    // TIMESTAMP -> DATE loses time component
    if old_upper.contains("TIMESTAMP") && new_upper.contains("DATE") && !new_upper.contains("TIMESTAMP") {
        return true;
    }

    false
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::migration::*;
    use crate::models::schema::SchemaState;
    use chrono::Utc;

    fn make_migration(id: &str, ops: Vec<SqlOperation>) -> Migration {
        Migration {
            id: id.to_string(),
            name: "test".to_string(),
            timestamp: Utc::now(),
            sql_content: String::new(),
            operations: ops,
            affected_tables: vec![],
        }
    }

    fn schema_with_user_table() -> SchemaState {
        let mut state = SchemaState::new();
        let m = make_migration("m0", vec![SqlOperation::CreateTable {
            table_name: "User".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    nullable: false,
                    default: None,
                },
                ColumnDefinition {
                    name: "email".to_string(),
                    data_type: "TEXT".to_string(),
                    nullable: true,
                    default: None,
                },
            ],
        }]);
        state.apply_migration(&m);
        state
    }

    #[test]
    fn test_non_nullable_without_default_on_existing_table() {
        let schema = schema_with_user_table();
        let m = make_migration("m1", vec![SqlOperation::AlterTable {
            table_name: "User".to_string(),
            alterations: vec![TableAlteration::AddColumn(ColumnDefinition {
                name: "age".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                default: None,
            })],
        }]);
        let findings = check_non_nullable_without_default(&m, &schema);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn test_non_nullable_with_default_is_ok() {
        let schema = schema_with_user_table();
        let m = make_migration("m1", vec![SqlOperation::AlterTable {
            table_name: "User".to_string(),
            alterations: vec![TableAlteration::AddColumn(ColumnDefinition {
                name: "age".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                default: Some("0".to_string()),
            })],
        }]);
        let findings = check_non_nullable_without_default(&m, &schema);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_non_nullable_on_new_table_is_ok() {
        let schema = SchemaState::new();
        let m = make_migration("m1", vec![SqlOperation::AlterTable {
            table_name: "NewTable".to_string(),
            alterations: vec![TableAlteration::AddColumn(ColumnDefinition {
                name: "col".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                default: None,
            })],
        }]);
        let findings = check_non_nullable_without_default(&m, &schema);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_destructive_drop_table() {
        let m = make_migration("m1", vec![SqlOperation::DropTable {
            table_name: "User".to_string(),
        }]);
        let findings = check_destructive_operations(&m);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn test_destructive_drop_column() {
        let m = make_migration("m1", vec![SqlOperation::AlterTable {
            table_name: "User".to_string(),
            alterations: vec![TableAlteration::DropColumn("email".to_string())],
        }]);
        let findings = check_destructive_operations(&m);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_missing_fk_index() {
        let mut schema = schema_with_user_table();
        let m = make_migration("m1", vec![
            SqlOperation::CreateTable {
                table_name: "Post".to_string(),
                columns: vec![ColumnDefinition {
                    name: "authorId".to_string(),
                    data_type: "TEXT".to_string(),
                    nullable: false,
                    default: None,
                }],
            },
            SqlOperation::AlterTable {
                table_name: "Post".to_string(),
                alterations: vec![TableAlteration::AddForeignKey {
                    constraint_name: "Post_authorId_fkey".to_string(),
                    columns: vec!["authorId".to_string()],
                    references_table: "User".to_string(),
                    references_columns: vec!["id".to_string()],
                }],
            },
        ]);
        schema.apply_migration(&m);
        let findings = check_missing_fk_index(&m, &schema);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("no index"));
    }

    #[test]
    fn test_fk_with_index_is_ok() {
        let mut schema = schema_with_user_table();
        let m = make_migration("m1", vec![
            SqlOperation::CreateTable {
                table_name: "Post".to_string(),
                columns: vec![],
            },
            SqlOperation::AlterTable {
                table_name: "Post".to_string(),
                alterations: vec![TableAlteration::AddForeignKey {
                    constraint_name: "Post_authorId_fkey".to_string(),
                    columns: vec!["authorId".to_string()],
                    references_table: "User".to_string(),
                    references_columns: vec!["id".to_string()],
                }],
            },
            SqlOperation::CreateIndex {
                index_name: "idx_post_author".to_string(),
                table_name: "Post".to_string(),
                columns: vec!["authorId".to_string()],
                unique: false,
            },
        ]);
        schema.apply_migration(&m);
        let findings = check_missing_fk_index(&m, &schema);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_type_change_lossy() {
        let schema = schema_with_user_table();
        let m = make_migration("m1", vec![SqlOperation::AlterTable {
            table_name: "User".to_string(),
            alterations: vec![TableAlteration::AlterColumn {
                column_name: "email".to_string(),
                new_type: Some("INTEGER".to_string()),
                new_nullable: None,
            }],
        }]);
        let findings = check_type_changes(&m, &schema);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_nullable_to_not_null() {
        let schema = schema_with_user_table();
        let m = make_migration("m1", vec![SqlOperation::AlterTable {
            table_name: "User".to_string(),
            alterations: vec![TableAlteration::AlterColumn {
                column_name: "email".to_string(),
                new_type: None,
                new_nullable: Some(false),
            }],
        }]);
        let findings = check_nullable_to_not_null(&m, &schema);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_unique_constraint_on_existing_table() {
        let schema = schema_with_user_table();
        let m = make_migration("m1", vec![SqlOperation::CreateIndex {
            index_name: "User_email_key".to_string(),
            table_name: "User".to_string(),
            columns: vec!["email".to_string()],
            unique: true,
        }]);
        let findings = check_add_unique_constraint(&m, &schema);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_is_potentially_lossy() {
        assert!(is_potentially_lossy_change("TEXT", "INTEGER"));
        assert!(is_potentially_lossy_change("BIGINT", "INTEGER"));
        assert!(is_potentially_lossy_change("TIMESTAMP", "DATE"));
        assert!(!is_potentially_lossy_change("INTEGER", "BIGINT"));
        assert!(!is_potentially_lossy_change("TEXT", "VARCHAR(255)"));
    }

    #[test]
    fn test_migration_summary() {
        let m = make_migration("m1", vec![
            SqlOperation::CreateTable {
                table_name: "User".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    nullable: false,
                    default: None,
                }],
            },
            SqlOperation::CreateIndex {
                index_name: "idx".to_string(),
                table_name: "User".to_string(),
                columns: vec!["id".to_string()],
                unique: true,
            },
        ]);
        let findings = migration_summary(&m);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
        assert!(findings[0].message.contains("Creates:"));
    }
}
