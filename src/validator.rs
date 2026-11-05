use std::collections::HashMap;
use crate::db::DbManager;
use crate::models::{Reto, ValidationResult};

/// Canonical value representation for comparison
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NormalizedValue {
    Null,
    Text(String),
}

impl NormalizedValue {
    fn from_str(s: &str) -> Self {
        let trimmed = s.trim();
        if trimmed.eq_ignore_ascii_case("null") {
            NormalizedValue::Null
        } else if let Ok(f) = trimmed.parse::<f64>() {
            // Round floats to 2 decimal places to allow tolerance for AVG, calculations
            NormalizedValue::Text(format!("{:.2}", f))
        } else {
            NormalizedValue::Text(trimmed.to_string())
        }
    }
}

pub struct Validator;

impl Validator {
    /// Validates user SQL against reference SQL following PDF rules (Page 10)
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

        // 5. Comparar datos fila por fila según si requiere orden o no
        if reto.requiere_orden {
            // Comparación secuencial estricta
            for (idx, (u_row, r_row)) in user_result.rows.iter().zip(ref_result.rows.iter()).enumerate() {
                if !Self::rows_equal(u_row, r_row) {
                    return ValidationResult {
                        is_valid: false,
                        title: "El orden o los datos de las filas difieren".to_string(),
                        message: format!(
                            "La fila {} no coincide con el orden esperado. Revisa la cláusula ORDER BY.",
                            idx + 1
                        ),
                        rows_matched: idx,
                        expected_rows: ref_result.total_rows,
                    };
                }
            }
        } else {
            // Comparación Multiset (bolsa de filas sin orden, respetando duplicados)
            let mut expected_map: HashMap<Vec<NormalizedValue>, usize> = HashMap::new();
            for r_row in &ref_result.rows {
                let norm: Vec<NormalizedValue> = r_row.iter().map(|v| NormalizedValue::from_str(v)).collect();
                *expected_map.entry(norm).or_insert(0) += 1;
            }

            let mut user_map: HashMap<Vec<NormalizedValue>, usize> = HashMap::new();
            for u_row in &user_result.rows {
                let norm: Vec<NormalizedValue> = u_row.iter().map(|v| NormalizedValue::from_str(v)).collect();
                *user_map.entry(norm).or_insert(0) += 1;
            }

            if expected_map != user_map {
                return ValidationResult {
                    is_valid: false,
                    title: "Los datos devueltos no coinciden".to_string(),
                    message: "Las filas devueltas no concuerdan con el conjunto de datos esperado para este reto.".to_string(),
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

    fn rows_equal(r1: &[String], r2: &[String]) -> bool {
        if r1.len() != r2.len() {
            return false;
        }
        for (v1, v2) in r1.iter().zip(r2.iter()) {
            let n1 = NormalizedValue::from_str(v1);
            let n2 = NormalizedValue::from_str(v2);
            if n1 != n2 {
                return false;
            }
        }
        true
    }
}
