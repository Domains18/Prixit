# Prixit - Prisma Migration Analyzer

A CLI tool that analyzes your Prisma migration history, detects potential issues, and provides optimization suggestions before you deploy to production.

## Installation

```bash
# Clone and build from source
git clone https://github.com/Domains18/Prixit.git
cd Prixit
cargo build --release

# Binary will be at target/release/migration-analyzer
```

## Usage

```bash
# Analyze migrations with colored console output
migration-analyzer analyze --path prisma/migrations

# Output as JSON (useful for CI/CD pipelines)
migration-analyzer analyze --path prisma/migrations --format json

# Uses default path (prisma/migrations) and format (console)
migration-analyzer analyze
```

### Exit Codes

- `0` — No errors found (warnings and info are OK)
- `1` — Error-level findings detected (migrations may fail on production)

This makes Prixit easy to integrate into CI pipelines as a migration safety gate.

### JSON Output

```json
{
  "total": 5,
  "errors": 1,
  "warnings": 2,
  "info": 2,
  "findings": [
    {
      "severity": "Error",
      "category": "Safety",
      "migration_id": "20231202120000_add_post",
      "table": "\"User\"",
      "message": "Non-nullable column '\"role\"' added to existing table '\"User\"' without a DEFAULT value.",
      "suggestion": "Add a DEFAULT value, or make the column nullable first, backfill data, then set NOT NULL."
    }
  ]
}
```

## What It Detects

| Rule | Severity | Description |
|------|----------|-------------|
| Non-nullable without default | Error | `ADD COLUMN NOT NULL` without `DEFAULT` on an existing table — will fail if rows exist |
| Destructive operations | Warning | `DROP TABLE` or `DROP COLUMN` — irreversible data loss |
| Missing FK index | Warning | Foreign key constraint without a corresponding index — slow JOINs |
| Lossy type changes | Warning | Column type changes that could lose data (e.g. `TEXT` to `INTEGER`) |
| Unique constraint on existing table | Warning | Adding a unique index that will fail if duplicates exist |
| Nullable to NOT NULL | Warning | Changing a column to NOT NULL without backfilling existing NULLs |
| Migration summary | Info | Per-migration summary of tables created, altered, and dropped |

## How It Works

1. **Discovery** — Scans your `prisma/migrations/` directory for migration folders (`YYYYMMDDHHMMSS_name/migration.sql`)
2. **Parsing** — Parses each `migration.sql` using a PostgreSQL SQL parser, extracting `CREATE TABLE`, `ALTER TABLE`, `CREATE INDEX`, `DROP` operations
3. **Schema Tracking** — Replays migrations in order, building an incremental schema state (tables, columns, indexes, foreign keys)
4. **Analysis** — Runs each migration through analysis rules, using the schema state to understand context (e.g. "does this table already exist?")
5. **Reporting** — Outputs findings sorted by severity in console (colored) or JSON format

## Project Structure

```
src/
├── main.rs              # CLI entry point and pipeline orchestration
├── error.rs             # Custom error types
├── cli/mod.rs           # Clap-based CLI definition
├── models/
│   ├── migration.rs     # Migration, SqlOperation, ColumnDefinition types
│   ├── finding.rs       # Finding, Severity, Category types
│   └── schema.rs        # SchemaState tracker
├── parser/
│   ├── migration.rs     # Migration directory discovery
│   └── sql.rs           # SQL statement parsing
├── analyzer/
│   ├── mod.rs           # Analysis orchestrator
│   └── rules.rs         # Analysis rules
└── reporter/
    └── mod.rs           # Console and JSON output formatters
```

## Development

```bash
# Run tests (31 tests: 28 unit + 3 integration)
cargo test

# Build in release mode
cargo build --release

# Run directly
cargo run -- analyze --path /path/to/prisma/migrations
```

## Future Plans

- Database introspection for schema drift detection (Prisma schema vs actual DB)
- Composite index recommendations based on query patterns
- Migration squashing suggestions for old migrations
- `init` command for project configuration
- Support for MySQL dialect
