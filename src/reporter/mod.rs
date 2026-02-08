use crate::models::{Finding, Severity};
use colored::*;

pub enum OutputFormat {
    Console,
    Json,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            _ => OutputFormat::Console,
        }
    }
}

pub fn report(findings: &[Finding], format: &OutputFormat) {
    match format {
        OutputFormat::Console => report_console(findings),
        OutputFormat::Json => report_json(findings),
    }
}

fn report_console(findings: &[Finding]) {
    if findings.is_empty() {
        println!("{}", "No findings. Your migrations look clean!".green().bold());
        return;
    }

    let errors = findings.iter().filter(|f| f.severity == Severity::Error).count();
    let warnings = findings.iter().filter(|f| f.severity == Severity::Warning).count();
    let infos = findings.iter().filter(|f| f.severity == Severity::Info).count();

    println!("\n{}", "=== Migration Analysis Report ===".bold());
    println!(
        "Found {} finding(s): {} error(s), {} warning(s), {} info(s)\n",
        findings.len(),
        errors.to_string().red().bold(),
        warnings.to_string().yellow().bold(),
        infos.to_string().bright_blue(),
    );

    for (i, finding) in findings.iter().enumerate() {
        let severity_str = match finding.severity {
            Severity::Error => "ERROR".red().bold(),
            Severity::Warning => "WARN ".yellow().bold(),
            Severity::Info => "INFO ".bright_blue(),
        };

        let category_str = format!("[{:?}]", finding.category).dimmed();

        println!("{}. {} {} {}", i + 1, severity_str, category_str, finding.message);

        if let Some(ref migration_id) = finding.migration_id {
            println!("   {} {}", "Migration:".dimmed(), migration_id);
        }
        if let Some(ref table) = finding.table {
            println!("   {} {}", "Table:".dimmed(), table);
        }
        if let Some(ref suggestion) = finding.suggestion {
            println!("   {} {}", "Suggestion:".green(), suggestion);
        }
        println!();
    }

    if errors > 0 {
        println!(
            "{}",
            format!("{} error(s) found. These migrations may fail on production.", errors)
                .red()
                .bold()
        );
    }
}

fn report_json(findings: &[Finding]) {
    let output = serde_json::json!({
        "total": findings.len(),
        "errors": findings.iter().filter(|f| f.severity == Severity::Error).count(),
        "warnings": findings.iter().filter(|f| f.severity == Severity::Warning).count(),
        "info": findings.iter().filter(|f| f.severity == Severity::Info).count(),
        "findings": findings,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use crate::models::{Finding, Severity, Category};

    #[test]
    fn test_json_serialization() {
        let findings = vec![
            Finding::new(Severity::Error, Category::Safety, "test error")
                .with_migration("m1")
                .with_table("User")
                .with_suggestion("fix it"),
        ];
        let json = serde_json::to_string(&findings).unwrap();
        assert!(json.contains("test error"));
        assert!(json.contains("fix it"));
    }
}
