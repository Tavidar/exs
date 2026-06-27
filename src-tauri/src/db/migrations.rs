// Перенесено из Exsul (src-tauri/src/db/migrations.rs) — robust migration
// runner. Список миграций пересобран под новое ядро (4 миграции вместо 29).
//
// The runner NEVER aborts the whole chain on a single failure: a bad migration
// is recorded in `failed_migrations` and the next one is attempted. This is the
// "flash-and-die" guarantee — a single bad migration must not stop the app from
// launching.

use rusqlite::Connection;

struct Migration {
    version: i32,
    name: &'static str,
    sql: &'static str,
    /// When true, the runner sets `PRAGMA foreign_keys = OFF` before running
    /// this migration and restores it afterwards. Needed when a migration
    /// rebuilds a parent table. PRAGMA foreign_keys is silently ignored inside
    /// a transaction, so it must be toggled BEFORE the wrapping BEGIN.
    needs_fk_off: bool,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "core_schema",
        sql: include_str!("../../migrations/001_core_schema.sql"),
        needs_fk_off: false,
    },
    Migration {
        version: 2,
        name: "projection_triggers",
        sql: include_str!("../../migrations/002_projection_triggers.sql"),
        needs_fk_off: false,
    },
    Migration {
        version: 3,
        name: "ai_metadata",
        sql: include_str!("../../migrations/003_ai_metadata.sql"),
        needs_fk_off: false,
    },
    Migration {
        version: 4,
        name: "search_fts",
        sql: include_str!("../../migrations/004_search_fts.sql"),
        needs_fk_off: false,
    },
    Migration {
        version: 5,
        name: "business_module",
        sql: include_str!("../../migrations/005_business_module.sql"),
        needs_fk_off: false,
    },
    Migration {
        // Deliberately outside the legacy Exsul range (1..32): old user
        // databases otherwise mistake the new core migrations for migrations
        // that were already applied by the donor application.
        version: 1001,
        name: "legacy_core_bridge",
        sql: include_str!("../../migrations/1001_legacy_core_bridge.sql"),
        needs_fk_off: false,
    },
];

/// Apply a single migration. First pass: whole file in one transaction
/// (all-or-nothing). Second pass (only on idempotent errors like "duplicate
/// column"/"already exists"): retry statement-by-statement so already-applied
/// parts can be skipped. `smart_split` respects BEGIN…END trigger bodies and
/// string literals so triggers stay intact.
fn apply_migration_safe(
    conn: &Connection,
    sql: &str,
    version: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let wrapped = format!("BEGIN IMMEDIATE;\n{}\nCOMMIT;", sql);

    match conn.execute_batch(&wrapped) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK;");
            let msg = e.to_string().to_lowercase();
            let is_idempotent =
                msg.contains("duplicate column name") || msg.contains("already exists");

            if !is_idempotent {
                return Err(Box::new(e));
            }

            log::warn!(
                "Migration {}: full-batch failed with idempotent error; \
                 retrying statement-by-statement: {}",
                version,
                e
            );
            apply_statement_by_statement(conn, sql, version)
        }
    }
}

/// Fallback: split into top-level statements and apply each. Idempotent
/// failures are skipped; any other failure aborts and is recorded upstream.
fn apply_statement_by_statement(
    conn: &Connection,
    sql: &str,
    version: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch("BEGIN IMMEDIATE;")?;

    for stmt in smart_split(sql) {
        let trimmed = stmt.trim();
        if trimmed.is_empty()
            || trimmed
                .lines()
                .all(|l| l.trim().is_empty() || l.trim().starts_with("--"))
        {
            continue;
        }

        let executable = format!("{};", trimmed);
        if let Err(e) = conn.execute_batch(&executable) {
            let msg = e.to_string().to_lowercase();
            let is_idempotent =
                msg.contains("duplicate column name") || msg.contains("already exists");
            if is_idempotent {
                log::warn!("Migration {}: skipping already-applied statement", version);
                continue;
            }
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(Box::new(e));
        }
    }

    conn.execute_batch("COMMIT;")?;
    Ok(())
}

/// Split a SQL script into top-level statements. Respects BEGIN…END blocks
/// (trigger bodies stay whole), single-quoted strings (`''` escapes), and
/// `--` / `/* */` comments. Intentionally simple — our migrations don't use
/// dollar-quoting.
fn smart_split(sql: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut in_str = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut begin_depth: i32 = 0;

    let bytes: &[u8] = sql.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        let next = if i + 1 < bytes.len() {
            bytes[i + 1] as char
        } else {
            '\0'
        };

        if in_line_comment {
            buf.push(c);
            if c == '\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if in_block_comment {
            buf.push(c);
            if c == '*' && next == '/' {
                buf.push(next);
                i += 2;
                in_block_comment = false;
                continue;
            }
            i += 1;
            continue;
        }
        if in_str {
            buf.push(c);
            if c == '\'' {
                if next == '\'' {
                    buf.push(next);
                    i += 2;
                    continue;
                }
                in_str = false;
            }
            i += 1;
            continue;
        }

        if c == '-' && next == '-' {
            in_line_comment = true;
            buf.push(c);
            i += 1;
            continue;
        }
        if c == '/' && next == '*' {
            in_block_comment = true;
            buf.push(c);
            buf.push(next);
            i += 2;
            continue;
        }
        if c == '\'' {
            in_str = true;
            buf.push(c);
            i += 1;
            continue;
        }

        // BEGIN / END detection — case-insensitive, word-boundary-aware
        if matches_keyword_at(bytes, i, b"BEGIN") {
            begin_depth += 1;
            buf.push_str("BEGIN");
            i += 5;
            continue;
        }
        if matches_keyword_at(bytes, i, b"END") {
            if begin_depth > 0 {
                begin_depth -= 1;
            }
            buf.push_str("END");
            i += 3;
            continue;
        }

        if c == ';' && begin_depth == 0 {
            out.push(std::mem::take(&mut buf));
            i += 1;
            continue;
        }

        buf.push(c);
        i += 1;
    }

    if !buf.trim().is_empty() {
        out.push(buf);
    }
    out
}

/// Whole-word (case-insensitive) keyword match at `bytes[pos]`.
fn matches_keyword_at(bytes: &[u8], pos: usize, kw: &[u8]) -> bool {
    if pos + kw.len() > bytes.len() {
        return false;
    }
    let slice = &bytes[pos..pos + kw.len()];
    if !slice
        .iter()
        .zip(kw.iter())
        .all(|(b, k)| b.to_ascii_uppercase() == *k)
    {
        return false;
    }
    let before_ok = pos == 0 || {
        let b = bytes[pos - 1];
        !(b.is_ascii_alphanumeric() || b == b'_')
    };
    let after_ok = pos + kw.len() == bytes.len() || {
        let b = bytes[pos + kw.len()];
        !(b.is_ascii_alphanumeric() || b == b'_')
    };
    before_ok && after_ok
}

/// Run all migrations. Returns the number of migrations that FAILED (recorded
/// in `failed_migrations`). The frontend reads that table to show a banner.
pub fn run(conn: &Connection) -> Result<usize, Box<dyn std::error::Error>> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    INTEGER PRIMARY KEY,
            name       TEXT    NOT NULL,
            applied_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
        );",
    )?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS failed_migrations (
            version      INTEGER NOT NULL,
            name         TEXT    NOT NULL,
            error        TEXT    NOT NULL,
            attempted_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
            PRIMARY KEY (version, attempted_at)
        );",
    )?;

    let mut failures: usize = 0;

    for migration in MIGRATIONS {
        let already_applied: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM schema_migrations WHERE version = ?1",
                [migration.version],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if already_applied {
            continue;
        }

        log::info!(
            "Applying migration {}: {}",
            migration.version,
            migration.name
        );

        let prior_fk: bool = if migration.needs_fk_off {
            let prior: i64 = conn
                .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
                .unwrap_or(0);
            let _ = conn.execute_batch("PRAGMA foreign_keys = OFF;");
            prior != 0
        } else {
            false
        };

        let result = apply_migration_safe(conn, migration.sql, migration.version);

        if migration.needs_fk_off {
            let restore_sql = if prior_fk {
                "PRAGMA foreign_keys = ON;"
            } else {
                "PRAGMA foreign_keys = OFF;"
            };
            let _ = conn.execute_batch(restore_sql);
        }

        match result {
            Ok(()) => {
                if let Err(e) = conn.execute(
                    "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
                    rusqlite::params![migration.version, migration.name],
                ) {
                    log::error!(
                        "Migration {} ({}) applied but bookkeeping insert failed: {}",
                        migration.version,
                        migration.name,
                        e
                    );
                    failures += 1;
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                log::error!(
                    "Migration {} ({}) FAILED: {} — continuing to next migration",
                    migration.version,
                    migration.name,
                    err_str
                );
                let _ = conn.execute(
                    "INSERT INTO failed_migrations (version, name, error) VALUES (?1, ?2, ?3)",
                    rusqlite::params![migration.version, migration.name, err_str],
                );
                failures += 1;
            }
        }
    }

    if failures == 0 {
        log::info!("All migrations applied");
    } else {
        log::warn!("Migrations completed with {} failure(s)", failures);
    }
    Ok(failures)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn
    }

    #[test]
    fn run_creates_core_tables() {
        let conn = fresh_conn();
        let failures = run(&conn).expect("migrations must apply");
        assert_eq!(failures, 0, "no migration should fail on a clean DB");

        for table in &[
            "events",
            "items",
            "item_prices",
            "item_photos",
            "categories",
            "local_config",
            "app_settings",
            "audit_logs",
            "ai_item_metadata",
            "item_search_fts",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name=?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "table '{}' must exist after migration", table);
        }
    }

    #[test]
    fn run_is_idempotent() {
        let conn = fresh_conn();
        run(&conn).unwrap();
        let failures = run(&conn).expect("second run must succeed");
        assert_eq!(failures, 0, "re-running migrations must be a no-op");
    }

    #[test]
    fn bridges_legacy_items_without_losing_rows() {
        let conn = fresh_conn();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT ''
            );
            INSERT INTO schema_migrations(version, name) VALUES
                (1, 'initial_schema'),
                (2, 'projection_triggers'),
                (3, 'extensions'),
                (4, 'preset_and_flowers');

            CREATE TABLE events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                aggregate_id TEXT NOT NULL,
                aggregate_type TEXT NOT NULL,
                event_type TEXT NOT NULL,
                data TEXT NOT NULL DEFAULT '{}',
                hlc_timestamp TEXT NOT NULL,
                node_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT ''
            );
            CREATE TABLE items (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'uncategorized',
                initial_price REAL NOT NULL DEFAULT 0.0,
                current_price REAL NOT NULL DEFAULT 0.0,
                production_cost REAL NOT NULL DEFAULT 0.0,
                current_stock INTEGER NOT NULL DEFAULT 0,
                sold_count INTEGER NOT NULL DEFAULT 0,
                revenue REAL NOT NULL DEFAULT 0.0,
                created_at TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT '',
                category_id TEXT,
                image_path TEXT,
                card_color TEXT
            );
            CREATE TABLE item_prices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
                price REAL NOT NULL,
                effective_at TEXT NOT NULL,
                event_id INTEGER NOT NULL REFERENCES events(id),
                created_at TEXT NOT NULL DEFAULT ''
            );
            INSERT INTO items (
                id, name, category, current_price, current_stock, created_at, updated_at
            ) VALUES ('legacy-1', 'Legacy tea', 'drinks', 12.5, 3, 'old', 'old');",
        )
        .unwrap();

        let failures = run(&conn).expect("legacy bridge must run");
        assert_eq!(failures, 0);

        let (name, description, attributes): (String, String, String) = conn
            .query_row(
                "SELECT name, description, attributes_json FROM items WHERE id='legacy-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(name, "Legacy tea");
        assert_eq!(description, "");
        assert_eq!(attributes, "{}");

        let indexed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM item_search_fts WHERE item_search_fts MATCH 'Legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(indexed, 1);
    }

    #[test]
    fn item_created_event_projects_into_items() {
        // The projection trigger (migration 002) must materialize an item row.
        let conn = fresh_conn();
        run(&conn).unwrap();

        conn.execute(
            "INSERT INTO events (aggregate_id, aggregate_type, event_type, data, hlc_timestamp, node_id, version)
             VALUES ('i1', 'item', 'ItemCreated', '{\"name\":\"ჩაი\",\"price\":12.5}', '0001-a', 'n', 1)",
            [],
        )
        .unwrap();

        let (name, price): (String, f64) = conn
            .query_row(
                "SELECT name, current_price FROM items WHERE id='i1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, "ჩაი");
        assert_eq!(price, 12.5);
    }

    #[test]
    fn smart_split_keeps_trigger_body_intact() {
        let sql = "CREATE TRIGGER t AFTER INSERT ON x BEGIN UPDATE y SET a=1; UPDATE y SET b=2; END; SELECT 1;";
        let parts = smart_split(sql);
        // Trigger (with its inner `;`) is one statement; the SELECT is another.
        assert_eq!(parts.len(), 2, "got: {:?}", parts);
    }
}
