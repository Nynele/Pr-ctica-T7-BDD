use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::{Connection, OpenFlags};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const EMBEDDED_DB: &[u8] = include_bytes!("../assets/davante_asir.db");

use crate::models::{CellValue, QueryResult, SchemaColumn, SchemaTable};

/// Strip leading whitespace and SQL comments (-- line and /* block */) to inspect the first keyword.
fn strip_leading_comments(mut s: &str) -> &str {
    loop {
        s = s.trim_start();
        if let Some(rest) = s.strip_prefix("--") {
            if let Some(nl) = rest.find('\n') {
                s = &rest[nl + 1..];
            } else {
                return "";
            }
        } else if let Some(rest) = s.strip_prefix("/*") {
            if let Some(end) = rest.find("*/") {
                s = &rest[end + 2..];
            } else {
                return "";
            }
        } else {
            return s;
        }
    }
}

/// Detect whether SQL contains more than one statement separated by `;` outside quotes and comments.
fn has_chained_statements(sql: &str) -> bool {
    let bytes = sql.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut saw_semicolon = false;

    while i < len {
        let b = bytes[i];
        if in_line_comment {
            if b == b'\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if in_block_comment {
            if b == b'*' && i + 1 < len && bytes[i + 1] == b'/' {
                in_block_comment = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if in_single {
            if b == b'\'' {
                if i + 1 < len && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if b == b'"' {
                if i + 1 < len && bytes[i + 1] == b'"' {
                    i += 2;
                    continue;
                }
                in_double = false;
            }
            i += 1;
            continue;
        }

        // Check comment starts outside strings
        if b == b'-' && i + 1 < len && bytes[i + 1] == b'-' {
            in_line_comment = true;
            i += 2;
            continue;
        }
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            in_block_comment = true;
            i += 2;
            continue;
        }

        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Any meaningful token after a semicolon indicates chained statements
        if saw_semicolon {
            return true;
        }

        if b == b'\'' {
            in_single = true;
        } else if b == b'"' {
            in_double = true;
        } else if b == b';' {
            saw_semicolon = true;
        }

        i += 1;
    }

    false
}

pub struct DbManager {
    db_path: String,
}

impl DbManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            db_path: path.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Resolve local database path or extract the embedded copy for standalone .exe distribution.
    pub fn resolve_portable_db() -> PathBuf {
        let cwd_db = PathBuf::from("assets/davante_asir.db");
        if cwd_db.exists() {
            return cwd_db;
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let near_assets = dir.join("assets").join("davante_asir.db");
                if near_assets.exists() {
                    return near_assets;
                }
                let near_root = dir.join("davante_asir.db");
                if near_root.exists() {
                    return near_root;
                }
            }
        }

        // Standalone .exe fallback: extract embedded DB into temp folder once
        let cache_dir = std::env::temp_dir().join("BD_Hector_Nerea_Deborah_db");
        let _ = fs::create_dir_all(&cache_dir);
        let cache_db = cache_dir.join("davante_asir.db");
        let needs_write = match fs::metadata(&cache_db) {
            Ok(meta) => meta.len() != EMBEDDED_DB.len() as u64,
            Err(_) => true,
        };
        if needs_write {
            let _ = fs::write(&cache_db, EMBEDDED_DB);
        }
        cache_db
    }

    /// Open a secure read-only SQLite connection with full authorizer restrictions
    pub fn open_read_only(&self) -> Result<Connection, String> {
        let uri = format!("file:{}?mode=ro", self.db_path);
        let conn = Connection::open_with_flags(
            &uri,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|_| "No se pudo abrir la base de datos educativa en modo solo lectura.".to_string())?;

        // Authorizer: Deny ALL write/modification actions
        conn.authorizer(Some(|auth_ctx: AuthContext| match auth_ctx.action {
            // Permitir exclusivamente SELECT, lectura de columnas y funciones SQL (COUNT, AVG, etc.)
            AuthAction::Select
            | AuthAction::Read { .. }
            | AuthAction::Function { .. } => Authorization::Allow,

            // Denegar cualquier otra acción (INSERT, UPDATE, DELETE, CREATE, DROP, ALTER, PRAGMA, ATTACH, etc.)
            _ => Authorization::Deny,
        }));

        Ok(conn)
    }

    /// Execute a SELECT query safely with a row display limit (50 rows for UI display)
    pub fn execute_query(&self, sql: &str, display_limit: usize) -> Result<QueryResult, String> {
        // 1. Comprobar entrada no vacía (incluyendo si solo hay comentarios iniciales)
        let cleaned = strip_leading_comments(sql).trim();
        if cleaned.is_empty() {
            return Err("La consulta está vacía. Introduce una sentencia SELECT válida.".to_string());
        }

        // 2. Verificar que comienza con SELECT válido (con límite de palabra)
        let upper = cleaned.to_uppercase();
        let starts_select = upper == "SELECT"
            || (upper.starts_with("SELECT")
                && upper.as_bytes().get(6).is_some_and(|b| {
                    b.is_ascii_whitespace() || *b == b'*' || *b == b'(' || *b == b'\'' || *b == b'"'
                }));

        if !starts_select {
            return Err(
                "Operación denegada: Solo se permiten consultas de lectura que comiencen con 'SELECT'."
                    .to_string(),
            );
        }

        // 3. Bloquear sentencias encadenadas con punto y coma fuera de literales/comentarios
        if has_chained_statements(cleaned) {
            return Err(
                "Seguridad: No se permiten múltiples sentencias encadenadas separadas por ';'."
                    .to_string(),
            );
        }

        let conn = self.open_read_only()?;
        let start = Instant::now();

        let mut stmt = conn
            .prepare(cleaned)
            .map_err(|e| format!("Error de sintaxis SQL: {}", e))?;

        let column_count = stmt.column_count();
        let column_names: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let mut rows = stmt
            .query([])
            .map_err(|e| format!("Error ejecutando consulta: {}", e))?;

        let mut result_rows = Vec::new();
        let mut typed_rows = Vec::new();
        let mut total_rows = 0;
        let mut is_truncated = false;

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("Error leyendo resultados: {}", e))?
        {
            total_rows += 1;
            if result_rows.len() < display_limit {
                let mut row_vals = Vec::with_capacity(column_count);
                let mut typed_vals = Vec::with_capacity(column_count);
                for i in 0..column_count {
                    let (str_val, cell_val) = match row.get_ref(i) {
                        Ok(rusqlite::types::ValueRef::Null) => {
                            ("NULL".to_string(), CellValue::Null)
                        }
                        Ok(rusqlite::types::ValueRef::Integer(n)) => {
                            (n.to_string(), CellValue::Integer(n))
                        }
                        Ok(rusqlite::types::ValueRef::Real(f)) => {
                            let s = if f.fract() == 0.0 {
                                format!("{:.1}", f)
                            } else {
                                format!("{:.2}", f)
                            };
                            (s, CellValue::Real(f))
                        }
                        Ok(rusqlite::types::ValueRef::Text(t)) => {
                            let s = String::from_utf8_lossy(t).to_string();
                            (s.clone(), CellValue::Text(s))
                        }
                        Ok(rusqlite::types::ValueRef::Blob(b)) => {
                            (format!("<BLOB {} bytes>", b.len()), CellValue::Blob(b.len()))
                        }
                        Err(_) => ("ERR".to_string(), CellValue::Text("ERR".to_string())),
                    };
                    row_vals.push(str_val);
                    typed_vals.push(cell_val);
                }
                result_rows.push(row_vals);
                typed_rows.push(typed_vals);
            } else {
                is_truncated = true;
            }
        }

        let duration = start.elapsed();
        let execution_time_ms = duration.as_secs_f64() * 1000.0;

        Ok(QueryResult {
            columns: column_names,
            rows: result_rows,
            typed_rows,
            total_rows,
            execution_time_ms,
            is_truncated,
        })
    }

    /// Extract complete schema of all tables and columns
    pub fn get_schema(&self) -> Result<Vec<SchemaTable>, String> {
        let conn = self.open_read_only()?;
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'actividades_evaluables' ORDER BY name",
            )
            .map_err(|e| format!("Error leyendo tablas: {}", e))?;

        let table_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| format!("Error mapeando tablas: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let mut schema = Vec::new();

        for table in table_names {
            let pragma_sql = format!("PRAGMA table_info(\"{}\")", table);
            // Open another direct connection without authorizer restricting pragma for schema inspection
            let info_conn = Connection::open_with_flags(
                &format!("file:{}?mode=ro", self.db_path),
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
            )
            .map_err(|e| e.to_string())?;

            let mut pragma_stmt = info_conn.prepare(&pragma_sql).map_err(|e| e.to_string())?;
            let columns = pragma_stmt
                .query_map([], |row| {
                    Ok(SchemaColumn {
                        name: row.get(1)?,
                        data_type: row.get(2)?,
                        not_null: row.get::<_, i32>(3)? != 0,
                        is_pk: row.get::<_, i32>(5)? != 0,
                    })
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            schema.push(SchemaTable {
                name: table,
                columns,
            });
        }

        Ok(schema)
    }
}
