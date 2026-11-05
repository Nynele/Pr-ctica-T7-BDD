use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::time::Instant;

use crate::models::{QueryResult, SchemaColumn, SchemaTable};

pub struct DbManager {
    db_path: String,
}

impl DbManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            db_path: path.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Open a secure read-only SQLite connection with full authorizer restrictions
    pub fn open_read_only(&self) -> Result<Connection, String> {
        let uri = format!("file:{}?mode=ro", self.db_path);
        let conn = Connection::open_with_flags(
            &uri,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|e| format!("Error abriendo base de datos en solo lectura: {}", e))?;

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

    /// Execute a SELECT query safely with a row display limit (default 100 rows for display)
    pub fn execute_query(&self, sql: &str, display_limit: usize) -> Result<QueryResult, String> {
        // Validación previa de palabras clave básicas para dar mensajes de error amigables
        let trimmed = sql.trim();
        let upper = trimmed.to_uppercase();

        if !upper.starts_with("SELECT") && !upper.starts_with("WITH") {
            return Err("Operación denegada: Solo se permiten consultas de lectura 'SELECT'.".to_string());
        }

        // Bloquear sentencias encadenadas con punto y coma
        let statement_count = trimmed.split(';').filter(|s| !s.trim().is_empty()).count();
        if statement_count > 1 {
            return Err("Seguridad: No se permiten múltiples sentencias encadenadas con ';'".to_string());
        }

        let conn = self.open_read_only()?;
        let start = Instant::now();

        let mut stmt = conn
            .prepare(trimmed)
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
        let mut total_rows = 0;
        let mut is_truncated = false;

        while let Some(row) = rows.next().map_err(|e| format!("Error leyendo fila: {}", e))? {
            total_rows += 1;
            if result_rows.len() < display_limit {
                let mut row_vals = Vec::with_capacity(column_count);
                for i in 0..column_count {
                    let val = match row.get_ref(i) {
                        Ok(rusqlite::types::ValueRef::Null) => "NULL".to_string(),
                        Ok(rusqlite::types::ValueRef::Integer(n)) => n.to_string(),
                        Ok(rusqlite::types::ValueRef::Real(f)) => {
                            if f.fract() == 0.0 {
                                format!("{:.1}", f)
                            } else {
                                format!("{:.2}", f)
                            }
                        }
                        Ok(rusqlite::types::ValueRef::Text(t)) => {
                            String::from_utf8_lossy(t).to_string()
                        }
                        Ok(rusqlite::types::ValueRef::Blob(b)) => {
                            format!("<BLOB {} bytes>", b.len())
                        }
                        Err(_) => "ERR".to_string(),
                    };
                    row_vals.push(val);
                }
                result_rows.push(row_vals);
            } else {
                is_truncated = true;
            }
        }

        let duration = start.elapsed();
        let execution_time_ms = duration.as_secs_f64() * 1000.0;

        Ok(QueryResult {
            columns: column_names,
            rows: result_rows,
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
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
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
