pub mod rules;

use crate::models::{Finding, Migration};
use crate::models::schema::SchemaState;

pub fn analyze(migrations: &[Migration]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut schema_state = SchemaState::new();

    for migration in migrations {
        // Rules that need schema state BEFORE this migration is applied
        findings.extend(rules::check_non_nullable_without_default(migration, &schema_state));
        findings.extend(rules::check_destructive_operations(migration));
        findings.extend(rules::check_type_changes(migration, &schema_state));
        findings.extend(rules::check_nullable_to_not_null(migration, &schema_state));
        findings.extend(rules::check_add_unique_constraint(migration, &schema_state));

        // Apply migration to advance schema state
        schema_state.apply_migration(migration);

        // Rules that need schema state AFTER this migration is applied
        findings.extend(rules::check_missing_fk_index(migration, &schema_state));

        // Informational
        findings.extend(rules::migration_summary(migration));
    }

    findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    findings
}
