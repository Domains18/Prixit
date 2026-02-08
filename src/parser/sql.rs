use crate::error::{AnalyzeError, Result};
use crate::models::{ColumnDefinition, SqlOperation, TableAlteration};
use sqlparser::ast::{AlterTableOperation, ColumnDef, Statement};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

pub struct SqlParser;

impl SqlParser {
    pub fn parse(sql: &str) -> Result<Vec<SqlOperation>> {
        let dialect = PostgreSqlDialect {};

        let statements = Parser::parse_sql(&dialect, sql)
            .map_err(|e| AnalyzeError::SqlParseError(e.to_string()))?;

        let mut operations = Vec::new();

        for statement in statements {
            if let Some(op) = Self::parse_statement(statement)? {
                operations.push(op);
            }
        }
        Ok(operations)
    }

    fn parse_statement(statement: Statement) -> Result<Option<SqlOperation>> {
        match statement {
            Statement::CreateTable(sqlparser::ast::CreateTable { name, columns, .. }) => {
                let table_name = name.to_string();
                let column_defs = columns.iter().map(Self::parse_column_def).collect();

                Ok(Some(SqlOperation::CreateTable {
                    table_name,
                    columns: column_defs,
                }))
            }

            Statement::AlterTable { name, operations, .. } => {
                let table_name = name.to_string();
                let alterations = operations
                    .iter()
                    .filter_map(|op| Self::parse_alter_operation(op))
                    .collect();

                Ok(Some(SqlOperation::AlterTable {
                    table_name,
                    alterations,
                }))
            }

            Statement::Drop { names, object_type, .. } => {
                if let Some(name) = names.first() {
                    match object_type {
                        sqlparser::ast::ObjectType::Index => {
                            Ok(Some(SqlOperation::DropIndex {
                                index_name: name.to_string(),
                            }))
                        }
                        _ => {
                            Ok(Some(SqlOperation::DropTable {
                                table_name: name.to_string(),
                            }))
                        }
                    }
                } else {
                    Ok(None)
                }
            }

            Statement::CreateIndex(sqlparser::ast::CreateIndex { name, table_name, columns, unique, .. }) => {
                let idx_name = name
                    .as_ref()
                    .map(|n| n.to_string())
                    .unwrap_or_default();

                let table = table_name.to_string();

                let column_names = columns
                    .iter()
                    .map(|col| col.expr.to_string())
                    .collect();

                Ok(Some(SqlOperation::CreateIndex {
                    index_name: idx_name,
                    table_name: table,
                    columns: column_names,
                    unique,
                }))
            }

            _ => Ok(None),
        }
    }

    fn parse_column_def(col: &ColumnDef) -> ColumnDefinition {
        let name = col.name.to_string();
        let data_type = col.data_type.to_string();

        let nullable = !col.options.iter().any(|opt| {
            matches!(opt.option, sqlparser::ast::ColumnOption::NotNull)
        });

        let default = col.options.iter().find_map(|opt| {
            if let sqlparser::ast::ColumnOption::Default(expr) = &opt.option {
                Some(expr.to_string())
            } else {
                None
            }
        });

        ColumnDefinition { name, data_type, nullable, default }
    }

    fn parse_alter_operation(op: &AlterTableOperation) -> Option<TableAlteration> {
        match op {
            AlterTableOperation::AddColumn { column_def, .. } => {
                Some(TableAlteration::AddColumn(Self::parse_column_def(column_def)))
            }

            AlterTableOperation::DropColumn { column_name, .. } => {
                Some(TableAlteration::DropColumn(column_name.to_string()))
            }

            AlterTableOperation::AlterColumn { column_name, op } => {
                let mut new_type = None;
                let mut new_nullable = None;

                match op {
                    sqlparser::ast::AlterColumnOperation::SetDataType { data_type, .. } => {
                        new_type = Some(data_type.to_string());
                    }
                    sqlparser::ast::AlterColumnOperation::SetNotNull => {
                        new_nullable = Some(false);
                    }
                    sqlparser::ast::AlterColumnOperation::DropNotNull => {
                        new_nullable = Some(true);
                    }
                    _ => {}
                }

                Some(TableAlteration::AlterColumn {
                    column_name: column_name.to_string(),
                    new_type,
                    new_nullable,
                })
            }

            AlterTableOperation::AddConstraint(constraint) => {
                if let sqlparser::ast::TableConstraint::ForeignKey {
                    name,
                    columns,
                    foreign_table,
                    referred_columns,
                    ..
                } = constraint
                {
                    Some(TableAlteration::AddForeignKey {
                        constraint_name: name
                            .as_ref()
                            .map(|n| n.to_string())
                            .unwrap_or_default(),
                        columns: columns.iter().map(|c| c.to_string()).collect(),
                        references_table: foreign_table.to_string(),
                        references_columns: referred_columns.iter().map(|c| c.to_string()).collect(),
                    })
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    pub fn extract_affected_tables(operations: &[SqlOperation]) -> Vec<String> {
        let mut tables = Vec::new();

        for op in operations {
            match op {
                SqlOperation::CreateTable { table_name, .. } => {
                    tables.push(table_name.clone());
                }
                SqlOperation::AlterTable { table_name, .. } => {
                    tables.push(table_name.clone());
                }
                SqlOperation::CreateIndex { table_name, .. } => {
                    tables.push(table_name.clone());
                }
                _ => {}
            }
        }

        tables.sort();
        tables.dedup();
        tables
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_create_table() {
        let sql = r#"
            CREATE TABLE "User"(
                "id" TEXT NOT NULL,
                "email" TEXT,
                PRIMARY KEY ("id")
            );
        "#;

        let operations = SqlParser::parse(sql).unwrap();
        assert_eq!(operations.len(), 1);

        if let SqlOperation::CreateTable { table_name, columns } = &operations[0] {
            assert_eq!(table_name, "\"User\"");
            assert_eq!(columns.len(), 2);
            assert_eq!(columns[0].name, "\"id\"");
            assert!(!columns[0].nullable);
        } else {
            panic!("Expected CreateTable operation");
        }
    }

    #[test]
    fn test_parse_alter_table_add_column() {
        let sql = r#"ALTER TABLE "User" ADD COLUMN "age" INTEGER NOT NULL DEFAULT 0;"#;
        let ops = SqlParser::parse(sql).unwrap();
        assert_eq!(ops.len(), 1);
        if let SqlOperation::AlterTable { table_name, alterations } = &ops[0] {
            assert_eq!(table_name, "\"User\"");
            assert_eq!(alterations.len(), 1);
            if let TableAlteration::AddColumn(col) = &alterations[0] {
                assert_eq!(col.name, "\"age\"");
                assert!(!col.nullable);
                assert_eq!(col.default, Some("0".to_string()));
            } else {
                panic!("Expected AddColumn");
            }
        } else {
            panic!("Expected AlterTable");
        }
    }

    #[test]
    fn test_parse_alter_column_set_type() {
        let sql = r#"ALTER TABLE "Post" ALTER COLUMN "title" SET DATA TYPE VARCHAR(500);"#;
        let ops = SqlParser::parse(sql).unwrap();
        if let SqlOperation::AlterTable { alterations, .. } = &ops[0] {
            if let TableAlteration::AlterColumn { new_type, .. } = &alterations[0] {
                assert!(new_type.is_some());
                assert!(new_type.as_ref().unwrap().contains("VARCHAR"));
            } else {
                panic!("Expected AlterColumn");
            }
        } else {
            panic!("Expected AlterTable");
        }
    }

    #[test]
    fn test_parse_alter_column_set_not_null() {
        let sql = r#"ALTER TABLE "Post" ALTER COLUMN "title" SET NOT NULL;"#;
        let ops = SqlParser::parse(sql).unwrap();
        if let SqlOperation::AlterTable { alterations, .. } = &ops[0] {
            if let TableAlteration::AlterColumn { new_nullable, .. } = &alterations[0] {
                assert_eq!(*new_nullable, Some(false));
            } else {
                panic!("Expected AlterColumn");
            }
        } else {
            panic!("Expected AlterTable");
        }
    }

    #[test]
    fn test_parse_foreign_key_constraint() {
        let sql = r#"ALTER TABLE "Post" ADD CONSTRAINT "Post_authorId_fkey" FOREIGN KEY ("authorId") REFERENCES "User"("id");"#;
        let ops = SqlParser::parse(sql).unwrap();
        if let SqlOperation::AlterTable { alterations, .. } = &ops[0] {
            if let TableAlteration::AddForeignKey { constraint_name, columns, references_table, .. } = &alterations[0] {
                assert!(constraint_name.contains("fkey"));
                assert_eq!(columns.len(), 1);
                assert!(references_table.contains("User"));
            } else {
                panic!("Expected AddForeignKey");
            }
        } else {
            panic!("Expected AlterTable");
        }
    }

    #[test]
    fn test_parse_drop_index() {
        let sql = r#"DROP INDEX "User_email_key";"#;
        let ops = SqlParser::parse(sql).unwrap();
        assert_eq!(ops.len(), 1);
        if let SqlOperation::DropIndex { index_name } = &ops[0] {
            assert!(index_name.contains("User_email_key"));
        } else {
            panic!("Expected DropIndex, got {:?}", ops[0]);
        }
    }

    #[test]
    fn test_parse_drop_table() {
        let sql = r#"DROP TABLE "User";"#;
        let ops = SqlParser::parse(sql).unwrap();
        assert_eq!(ops.len(), 1);
        if let SqlOperation::DropTable { table_name } = &ops[0] {
            assert!(table_name.contains("User"));
        } else {
            panic!("Expected DropTable, got {:?}", ops[0]);
        }
    }

    #[test]
    fn test_extract_affected_tables() {
        let ops = vec![
            SqlOperation::CreateTable {
                table_name: "User".to_string(),
                columns: vec![],
            },
            SqlOperation::AlterTable {
                table_name: "User".to_string(),
                alterations: vec![],
            },
            SqlOperation::CreateIndex {
                index_name: "idx".to_string(),
                table_name: "Post".to_string(),
                columns: vec![],
                unique: false,
            },
        ];
        let tables = SqlParser::extract_affected_tables(&ops);
        assert_eq!(tables, vec!["Post", "User"]);
    }
}
