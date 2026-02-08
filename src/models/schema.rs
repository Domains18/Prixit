use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::migration::{ColumnDefinition, SqlOperation, TableAlteration, Migration};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaState {
    pub tables: HashMap<String, TableSchema>,
    pub indexes: HashMap<String, IndexInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub columns: HashMap<String, ColumnDefinition>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
    pub created_in: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyInfo {
    pub constraint_name: String,
    pub columns: Vec<String>,
    pub references_table: String,
    pub references_columns: Vec<String>,
}

impl SchemaState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_migration(&mut self, migration: &Migration) {
        for op in &migration.operations {
            self.apply_operation(op, &migration.id);
        }
    }

    fn apply_operation(&mut self, op: &SqlOperation, migration_id: &str) {
        match op {
            SqlOperation::CreateTable { table_name, columns } => {
                let mut col_map = HashMap::new();
                for col in columns {
                    col_map.insert(col.name.clone(), col.clone());
                }
                self.tables.insert(
                    table_name.clone(),
                    TableSchema {
                        name: table_name.clone(),
                        columns: col_map,
                        foreign_keys: Vec::new(),
                        created_in: migration_id.to_string(),
                    },
                );
            }

            SqlOperation::AlterTable { table_name, alterations } => {
                if let Some(table) = self.tables.get_mut(table_name) {
                    for alteration in alterations {
                        match alteration {
                            TableAlteration::AddColumn(col_def) => {
                                table.columns.insert(col_def.name.clone(), col_def.clone());
                            }
                            TableAlteration::DropColumn(col_name) => {
                                table.columns.remove(col_name);
                            }
                            TableAlteration::AlterColumn { column_name, new_type, new_nullable } => {
                                if let Some(col) = table.columns.get_mut(column_name) {
                                    if let Some(t) = new_type {
                                        col.data_type = t.clone();
                                    }
                                    if let Some(n) = new_nullable {
                                        col.nullable = *n;
                                    }
                                }
                            }
                            TableAlteration::AddForeignKey {
                                constraint_name, columns,
                                references_table, references_columns,
                            } => {
                                table.foreign_keys.push(ForeignKeyInfo {
                                    constraint_name: constraint_name.clone(),
                                    columns: columns.clone(),
                                    references_table: references_table.clone(),
                                    references_columns: references_columns.clone(),
                                });
                            }
                        }
                    }
                }
            }

            SqlOperation::CreateIndex { index_name, table_name, columns, unique } => {
                self.indexes.insert(
                    index_name.clone(),
                    IndexInfo {
                        name: index_name.clone(),
                        table_name: table_name.clone(),
                        columns: columns.clone(),
                        unique: *unique,
                    },
                );
            }

            SqlOperation::DropTable { table_name } => {
                self.tables.remove(table_name);
                self.indexes.retain(|_, idx| &idx.table_name != table_name);
            }

            SqlOperation::DropIndex { index_name } => {
                self.indexes.remove(index_name);
            }
        }
    }

    pub fn has_index_on(&self, table_name: &str, columns: &[String]) -> bool {
        self.indexes.values().any(|idx| {
            idx.table_name == table_name
                && columns.iter().all(|col| idx.columns.contains(col))
        })
    }

    pub fn table_exists(&self, table_name: &str) -> bool {
        self.tables.contains_key(table_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_migration(id: &str, operations: Vec<SqlOperation>) -> Migration {
        Migration {
            id: id.to_string(),
            name: "test".to_string(),
            timestamp: Utc::now(),
            sql_content: String::new(),
            operations,
            affected_tables: vec![],
        }
    }

    #[test]
    fn test_create_table_tracked() {
        let mut state = SchemaState::new();
        let m = make_migration("m1", vec![SqlOperation::CreateTable {
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
        assert!(state.table_exists("User"));
        assert_eq!(state.tables["User"].columns.len(), 2);
    }

    #[test]
    fn test_alter_table_add_column() {
        let mut state = SchemaState::new();
        let m1 = make_migration("m1", vec![SqlOperation::CreateTable {
            table_name: "User".to_string(),
            columns: vec![],
        }]);
        let m2 = make_migration("m2", vec![SqlOperation::AlterTable {
            table_name: "User".to_string(),
            alterations: vec![TableAlteration::AddColumn(ColumnDefinition {
                name: "age".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                default: Some("0".to_string()),
            })],
        }]);
        state.apply_migration(&m1);
        state.apply_migration(&m2);
        assert!(state.tables["User"].columns.contains_key("age"));
    }

    #[test]
    fn test_drop_table_removes_indexes() {
        let mut state = SchemaState::new();
        let m1 = make_migration("m1", vec![
            SqlOperation::CreateTable {
                table_name: "User".to_string(),
                columns: vec![],
            },
            SqlOperation::CreateIndex {
                index_name: "idx_user_email".to_string(),
                table_name: "User".to_string(),
                columns: vec!["email".to_string()],
                unique: false,
            },
        ]);
        let m2 = make_migration("m2", vec![SqlOperation::DropTable {
            table_name: "User".to_string(),
        }]);
        state.apply_migration(&m1);
        assert!(state.indexes.contains_key("idx_user_email"));
        state.apply_migration(&m2);
        assert!(!state.table_exists("User"));
        assert!(state.indexes.is_empty());
    }

    #[test]
    fn test_has_index_on() {
        let mut state = SchemaState::new();
        let m = make_migration("m1", vec![
            SqlOperation::CreateTable {
                table_name: "Post".to_string(),
                columns: vec![],
            },
            SqlOperation::CreateIndex {
                index_name: "idx_post_author".to_string(),
                table_name: "Post".to_string(),
                columns: vec!["authorId".to_string()],
                unique: false,
            },
        ]);
        state.apply_migration(&m);
        assert!(state.has_index_on("Post", &["authorId".to_string()]));
        assert!(!state.has_index_on("Post", &["title".to_string()]));
    }

    #[test]
    fn test_alter_column_updates_type() {
        let mut state = SchemaState::new();
        let m1 = make_migration("m1", vec![SqlOperation::CreateTable {
            table_name: "User".to_string(),
            columns: vec![ColumnDefinition {
                name: "age".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: true,
                default: None,
            }],
        }]);
        let m2 = make_migration("m2", vec![SqlOperation::AlterTable {
            table_name: "User".to_string(),
            alterations: vec![TableAlteration::AlterColumn {
                column_name: "age".to_string(),
                new_type: Some("BIGINT".to_string()),
                new_nullable: Some(false),
            }],
        }]);
        state.apply_migration(&m1);
        state.apply_migration(&m2);
        let col = &state.tables["User"].columns["age"];
        assert_eq!(col.data_type, "BIGINT");
        assert!(!col.nullable);
    }
}
