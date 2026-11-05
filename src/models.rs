use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reto {
    pub id: String,
    pub nivel: u8,
    pub titulo: String,
    pub peticion: String,
    pub tabla: String,
    pub elementos_sql: String,
    pub consulta_referencia: String,
    pub requiere_orden: bool,
    pub pistas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_rows: usize,
    pub execution_time_ms: f64,
    pub is_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub title: String,
    pub message: String,
    pub rows_matched: usize,
    pub expected_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentoRegistro {
    pub reto_id: String,
    pub sql: String,
    pub timestamp: String,
    pub tiempo_segundos: u64,
    pub superado: bool,
    pub mensaje_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SesionResumen {
    pub sesion_id: String,
    pub equipo_codigo: String,
    pub fecha_inicio: String,
    pub fecha_fin: Option<String>,
    pub retos_completados: usize,
    pub total_retos: usize,
    pub total_intentos: usize,
    pub tiempo_total_segundos: u64,
    pub intentos: Vec<IntentoRegistro>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaColumn {
    pub name: String,
    pub data_type: String,
    pub is_pk: bool,
    pub not_null: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaTable {
    pub name: String,
    pub columns: Vec<SchemaColumn>,
}
