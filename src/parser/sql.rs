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
            Statement::CreateTable { name, columns, .. } => {
                let table_name = name.to_string();
                let colum_defs = columns.iter().map(Self::parse_column_def).collect();

                Ok(Some(SqlOperation::CreateTable {
                    table_name,
                    columns: colum_defs,
                }))
            }

            Statement::AlterTable { name, operations } => {
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
                name,
                table_name,
                columns,
                unique,
                ..
            } => {
                let index_name = name.map(|n| n.to_string()).unwrap_or_default();
                let table = table_name.toString();

                let column_names = columns.iter().map(|col| col.to_string()).collect();

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
            if let sqlparser::ast::ColumnOption::Default(expr) => &opt.option{
                Some(expr.to_string())
            } else {
                None
            }
        });

        ColumnDefinition { name, data_type, nullable, default }
    }

    fn parse_alter_operation(op: &AlterTableOperation) -> Option<TableALteration>{
        match op {
            AlterTableOperation::AddColumn{column_def, ..}=>{
                Some(TableALteration::AddColumn(Self::parse_column_def(column_def)))
            }

            AlterTableOperation::DropColumn{column_name, ..}=>{
                Some(TableALteration::DropColumn(column_name.to_string()))
            }

            AlterTableOperation::AlterColumn { column_name, op }=>{
                Some(TableALteration::AlterColumn { column_name: column_name.to_string(), new_type: None, new_nullable: None })
            }
            _ => None,
        }
    }


    pub fn extract_affected_tables(operations: &[SqlOperation]) -> Vec<String>{
        let mut tables = Vec::new()

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
