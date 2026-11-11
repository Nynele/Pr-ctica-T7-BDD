use std::collections::HashMap;
use crate::db::DbManager;
use crate::models::{CellValue, Reto, ValidationResult};

/// Typed canonical value representation for accurate comparison of types & logical values
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NormalizedValue {
    Null,
    Number(i128), // Scaled by 100 to compare INTEGER and REAL logically while keeping type strictness vs Text
    Text(String),
    Blob(usize),
}

impl NormalizedValue {
    fn from_cell(cell: &CellValue) -> Self {
        match cell {
            CellValue::Null => NormalizedValue::Null,
            CellValue::Integer(n) => NormalizedValue::Number((*n as i128) * 100),
            CellValue::Real(f) => NormalizedValue::Number((*f * 100.0).round() as i128),
            CellValue::Text(s) => NormalizedValue::Text(s.trim().to_string()),
            CellValue::Blob(len) => NormalizedValue::Blob(*len),
        }
    }
}

pub struct Validator;

impl Validator {
    /// Validates user SQL against reference SQL checking columns, rows, types, and values.
    pub fn validate(db: &DbManager, reto: &Reto, user_sql: &str) -> ValidationResult {
        // 1. Ejecutar consulta de referencia (sin límite de visualización para comparar la totalidad de los datos)
        let ref_result = match db.execute_query(&reto.consulta_referencia, usize::MAX) {
            Ok(res) => res,
            Err(e) => {
                return ValidationResult {
                    is_valid: false,
                    title: "Error interno de referencia".to_string(),
                    message: format!("Error en la consulta canónica: {}", e),
                    rows_matched: 0,
                    expected_rows: 0,
                };
            }
        };

        // 2. Ejecutar consulta del usuario
        let user_result = match db.execute_query(user_sql, usize::MAX) {
            Ok(res) => res,
            Err(e) => {
                return ValidationResult {
                    is_valid: false,
                    title: "Error de ejecución SQL".to_string(),
                    message: e,
                    rows_matched: 0,
                    expected_rows: ref_result.total_rows,
                };
            }
        };

        // 3. Comparar número de columnas
        if user_result.columns.len() != ref_result.columns.len() {
            return ValidationResult {
                is_valid: false,
                title: "Número de columnas incorrecto".to_string(),
                message: format!(
                    "Tu consulta devuelve {} columna(s), pero se esperaban {}.",
                    user_result.columns.len(),
                    ref_result.columns.len()
                ),
                rows_matched: 0,
                expected_rows: ref_result.total_rows,
            };
        }

        // 4. Comparar número de filas
        if user_result.total_rows != ref_result.total_rows {
            return ValidationResult {
                is_valid: false,
                title: "Número de filas devueltas no coincide".to_string(),
                message: format!(
                    "Tu consulta devolvió {} filas, pero el resultado esperado contiene {} filas.",
                    user_result.total_rows, ref_result.total_rows
                ),
                rows_matched: 0,
                expected_rows: ref_result.total_rows,
            };
        }

        // 5. Comparar tipos y valores fila por fila según si requiere orden o no
        if reto.requiere_orden {
            // Comparación secuencial estricta
            for (idx, (u_row, r_row)) in user_result
                .typed_rows
                .iter()
                .zip(ref_result.typed_rows.iter())
                .enumerate()
            {
                if !Self::rows_equal(u_row, r_row) {
                    return ValidationResult {
                        is_valid: false,
                        title: "El orden, los tipos o los datos difieren".to_string(),
                        message: format!(
                            "La fila {} no coincide con el resultado esperado. Revisa la cláusula ORDER BY o las columnas seleccionadas.",
                            idx + 1
                        ),
                        rows_matched: idx,
                        expected_rows: ref_result.total_rows,
                    };
                }
            }
        } else {
            // Comparación Multiset (bolsa de filas sin orden, respetando duplicados y tipos)
            let mut expected_map: HashMap<Vec<NormalizedValue>, usize> = HashMap::new();
            for r_row in &ref_result.typed_rows {
                let norm: Vec<NormalizedValue> =
                    r_row.iter().map(NormalizedValue::from_cell).collect();
                *expected_map.entry(norm).or_insert(0) += 1;
            }

            let mut user_map: HashMap<Vec<NormalizedValue>, usize> = HashMap::new();
            for u_row in &user_result.typed_rows {
                let norm: Vec<NormalizedValue> =
                    u_row.iter().map(NormalizedValue::from_cell).collect();
                *user_map.entry(norm).or_insert(0) += 1;
            }

            if expected_map != user_map {
                return ValidationResult {
                    is_valid: false,
                    title: "Los datos o tipos devueltos no coinciden".to_string(),
                    message: "Las filas devueltas no concuerdan con el conjunto de datos y tipos esperado para este reto.".to_string(),
                    rows_matched: 0,
                    expected_rows: ref_result.total_rows,
                };
            }
        }

        // Si pasó todas las comprobaciones
        ValidationResult {
            is_valid: true,
            title: "¡Consulta Correcta!".to_string(),
            message: format!(
                "Resultado validado con éxito. Se han comprobado {} filas.",
                ref_result.total_rows
            ),
            rows_matched: ref_result.total_rows,
            expected_rows: ref_result.total_rows,
        }
    }

    fn rows_equal(r1: &[CellValue], r2: &[CellValue]) -> bool {
        if r1.len() != r2.len() {
            return false;
        }
        r1.iter()
            .zip(r2.iter())
            .all(|(c1, c2)| NormalizedValue::from_cell(c1) == NormalizedValue::from_cell(c2))
    }
}
