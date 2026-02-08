use std::fs;
use std::path::PathBuf;

fn create_test_migrations(base: &PathBuf) {
    // Migration 1: Create User table
    let m1_dir = base.join("20231201120000_init");
    fs::create_dir_all(&m1_dir).unwrap();
    fs::write(
        m1_dir.join("migration.sql"),
        r#"
        CREATE TABLE "User" (
            "id" TEXT NOT NULL,
            "email" TEXT,
            PRIMARY KEY ("id")
        );
        "#,
    )
    .unwrap();

    // Migration 2: Add Post table with FK (no index) + add NOT NULL column without default to User
    let m2_dir = base.join("20231202120000_add_post");
    fs::create_dir_all(&m2_dir).unwrap();
    fs::write(
        m2_dir.join("migration.sql"),
        r#"
        CREATE TABLE "Post" (
            "id" TEXT NOT NULL,
            "title" TEXT NOT NULL,
            "authorId" TEXT NOT NULL,
            PRIMARY KEY ("id")
        );

        ALTER TABLE "Post" ADD CONSTRAINT "Post_authorId_fkey"
            FOREIGN KEY ("authorId") REFERENCES "User"("id");

        ALTER TABLE "User" ADD COLUMN "role" TEXT NOT NULL;
        "#,
    )
    .unwrap();
}

#[test]
fn test_full_analysis_json_output() {
    let tmp = std::env::temp_dir().join("prixit_test_integration");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    create_test_migrations(&tmp);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_migration-analyzer"))
        .args(["analyze", "--path", tmp.to_str().unwrap(), "--format", "json"])
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain JSON output with findings
    assert!(stdout.contains("\"findings\""), "Expected JSON output with findings key");
    assert!(stdout.contains("\"errors\""), "Expected errors count in JSON");

    // Should detect non-nullable without default (Error severity)
    assert!(stdout.contains("Non-nullable column"), "Expected non-nullable column finding");

    // Should detect missing FK index (Warning severity)
    assert!(stdout.contains("no index"), "Expected missing FK index finding");

    // Should have migration summaries (Info severity)
    assert!(stdout.contains("Creates:"), "Expected migration summary");

    // Exit code should be 1 because there's an Error-severity finding
    assert!(!output.status.success(), "Expected non-zero exit code due to Error findings");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_analysis_console_output() {
    let tmp = std::env::temp_dir().join("prixit_test_console");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    create_test_migrations(&tmp);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_migration-analyzer"))
        .args(["analyze", "--path", tmp.to_str().unwrap(), "--format", "console"])
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Console output should have the report header
    assert!(stdout.contains("Migration Analysis Report"), "Expected report header");
    assert!(stdout.contains("finding(s)"), "Expected findings count");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_empty_migrations_dir() {
    let tmp = std::env::temp_dir().join("prixit_test_empty");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_migration-analyzer"))
        .args(["analyze", "--path", tmp.to_str().unwrap()])
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No migrations found"), "Expected 'no migrations' message");
    assert!(output.status.success(), "Expected zero exit code for empty dir");

    let _ = fs::remove_dir_all(&tmp);
}
