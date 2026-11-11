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

impl Reto {
    /// Puntos asignados según el nivel de dificultad
    pub fn puntos(&self) -> u32 {
        match self.nivel {
            1 => 10,
            2 => 20,
            3 => 30,
            4 => 40,
            5 => 50,
            _ => 10,
        }
    }

    /// Categoría temática oficial según el temario ASIR
    pub fn categoria(&self) -> &'static str {
        match self.nivel {
            1 => "Selección y Proyección",
            2 => "Filtrado con WHERE",
            3 => "Ordenación y Límites",
            4 => "Resumen y Agregación",
            5 => "Agrupación Avanzada (HAVING)",
            _ => "General",
        }
    }

    /// Etiqueta descriptiva de dificultad
    pub fn dificultad_label(&self) -> &'static str {
        match self.nivel {
            1 => "BÁSICO",
            2 => "FÁCIL",
            3 => "INTERMEDIO",
            4 => "AVANZADO",
            5 => "EXPERTO",
            _ => "NORMAL",
        }
    }

    /// Color hexadecimal para identificación visual rápida
    pub fn color_hex(&self) -> u32 {
        match self.nivel {
            1 => 0x10b981, // Verde Esmeralda (N1)
            2 => 0x06b6d4, // Cian / Azul Cielo (N2)
            3 => 0x8b5cf6, // Violeta / Púrpura (N3)
            4 => 0xf59e0b, // Ámbar / Naranja (N4)
            5 => 0xef4444, // Rojo Coral (N5)
            _ => 0x38bdf8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CellValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    #[serde(skip)]
    pub typed_rows: Vec<Vec<CellValue>>,
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
    pub puntos_obtenidos: u32,
    pub mensaje_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SesionResumen {
    pub sesion_id: String,
    pub participante: String,
    pub fecha_inicio: String,
    pub fecha_fin: Option<String>,
    pub retos_completados: usize,
    pub retos_no_superados: usize,
    pub total_retos: usize,
    pub total_intentos: usize,
    pub errores_cometidos: usize,
    pub puntos_obtenidos: u32,
    pub puntos_totales_posibles: u32,
    pub tiempo_total_segundos: u64,
    pub tiempo_restante_global: u32,
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
