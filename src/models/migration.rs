use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    pub id: String,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub sql_content: String,
    pub operations: Vec<SqlOperation>,
    pub affected_tables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SqlOperation {
    CreateTable {
        table_name: String,
        columns: Vec<ColumnDefinition>,
    },

    AlterTable {
        table_name: String,
        alterations: Vec<TableAlteration>,
    },

    CreateIndex {
        index_name: String,
        table_name: String,
        columns: Vec<String>,
        unique: bool,
    },

    DropTable{
        table_name: String
    },

    DropIndex {
        index_name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TableAlteration {
    AddColumn(ColumnDefinition),
    DropColumn(String),
    AlterColumn {
        column_name: String,
        new_type: String,
        new_nullable: Option<bool>
    },
    AddForeignKey {
        constraint_name: String,
        columns: Vec<String>,
        references_table: String,
        references_columns: Vec<String>,
    },
}
