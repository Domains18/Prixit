use crate::error::{AnalyzeError, Result};
use crate::models::{ColumnDefinition, SqlOperation, TableAlteration};
use sqlparser::ast::{AlterTableOperation, ColumnDef, Statement};
use sqlparser::dialect::{self, PostgreSqlDialect};
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
            Statement::CreateTable { .. } => {
                let table_name = name.to_string();
                let colum_defs = columns.iter().map(Self::parse_column_def).collect();

                Ok(Some(SqlOperation::CreateTable {
                    table_name,
                    columns: colum_defs,
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

            Statement::Drop { names, .. } => {
                if let Some(name) = names.first() {
                    Ok(Some(SqlOperation::DropTable {
                        table_name: name.to_string(),
                    }))
                } else {
                    Ok(None)
                }
            }

            Statement::CreateIndex {
                ..
            } => {
                let index_name = name
                    .map(|n| n.to_string())
                    .unwrap_or_default();
            
                let table = table_name.to_string();
            
                let column_names = columns
                    .iter()
                    .map(|col| col.expr.to_string())
                    .collect();
            
                Ok(Some(SqlOperation::CreateIndex {
                    index_name,
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
            if let sqlparser::ast::ColumnOption::Default(expr) = &opt.option{
                Some(expr.to_string())
            } else {
                None
            }
        });

        ColumnDefinition { name, data_type, nullable, default }
    }

    fn parse_alter_operation(op: &AlterTableOperation) -> Option<TableAlteration>{
        match op {
            AlterTableOperation::AddColumn{column_def, ..}=>{
                Some(TableAlteration::AddColumn(Self::parse_column_def(column_def)))
            }

            AlterTableOperation::DropColumn{column_name, ..}=>{
                Some(TableAlteration::DropColumn(column_name.to_string()))
            }

            AlterTableOperation::AlterColumn { column_name, op }=>{
                Some(TableAlteration::AlterColumn { column_name: column_name.to_string(), new_type: None, new_nullable: None, })
            }
            _ => None,
        }
    }


    pub fn extract_affected_tables(operations: &[SqlOperation]) -> Vec<String>{
        let mut tables = Vec::new();

        for op in operations {
            match op {
                SqlOperation::CreateTable { table_name, .. } =>{
                    tables.push(table_name.clone());
                }

                SqlOperation::AlterTable { table_name, .. } =>{
                    tables.push(table_name.clone());
                }

                SqlOperation::CreateIndex{table_name, ..} =>{
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
    fn parse_parse_create_table(){
        let sql = r#"
            CREATE TABLE "User"(
                "id" TEXT NOT NULL,
                "email" TEXT,
                PRIMARY KEY ("id")
            );
        "#;

        let operations = SqlParser::parse(sql).unwrap();
        assert_eq!(operations.len(), 1);
        

        if let SqlOperation::CreateTable { table_name, columns } = &operations[0]{
            assert_eq!(table_name, "\"User\"");
            assert_eq!(columns.len(), 3);
            assert_eq!(columns[0].name, "id");
            assert!(!columns[0].nullable);
        } else {
            panic!("Expected CreateTable Operations");
        }
    }
}