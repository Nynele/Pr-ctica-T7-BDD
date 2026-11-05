use std::collections::HashMap;
use std::fs;
use uuid::Uuid;

use crate::db::DbManager;
use crate::export::Exporter;
use crate::models::{
    IntentoRegistro, QueryResult, Reto, SchemaTable, SesionResumen, ValidationResult,
};
use crate::validator::Validator;

pub struct SimuladorState {
    pub db: DbManager,
    pub retos: Vec<Reto>,
    pub reto_actual_idx: usize,
    pub equipo_codigo: String,
    pub participante_nombre: String,
    pub sesion_id: String,
    pub fecha_inicio: String,

    // Temporizador (5 minutos por reto = 300 segundos)
    pub tiempo_restante_segundos: u32,
    pub temporizador_activo: bool,
    pub tiempo_transcurrido_reto: u64,

    // Editor SQL
    pub sql_buffer: String,

    // Resultados
    pub query_result: Option<QueryResult>,
    pub validation_result: Option<ValidationResult>,
    pub feedback_banner: Option<(bool, String)>, // (es_exito, mensaje)

    // Esquema de la BD
    pub schema: Vec<SchemaTable>,
    pub tablas_expandidas: HashMap<String, bool>,

    // Progreso y métricas
    pub retos_superados: Vec<String>, // IDs de retos acertados
    pub intentos_por_reto: HashMap<String, usize>,
    pub historial_intentos: Vec<IntentoRegistro>,
    pub tiempo_total_segundos: u64,

    // Modales y vistas
    pub mostrar_modal_ranking: bool,
    pub mostrar_pistas: bool,
    pub mensaje_sistema: Option<String>,
}

impl SimuladorState {
    pub fn new() -> Self {
        let db_path = "assets/davante_asir.db";
        let retos_path = "assets/retos.json";

        let db = DbManager::new(db_path);

        // Cargar 30 retos desde JSON externo como exige el PDF
        let retos: Vec<Reto> = match fs::read_to_string(retos_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        // Cargar esquema completo de la base de datos
        let schema = db.get_schema().unwrap_or_default();
        let mut tablas_expandidas = HashMap::new();
        for t in &schema {
            tablas_expandidas.insert(t.name.clone(), false);
        }
        // Expandir por defecto la tabla del primer reto
        tablas_expandidas.insert("alumnos".to_string(), true);

        let initial_sql = if !retos.is_empty() {
            format!("-- Reto {}: {}\nSELECT ", retos[0].id, retos[0].titulo)
        } else {
            "SELECT ".to_string()
        };

        Self {
            db,
            retos,
            reto_actual_idx: 0,
            equipo_codigo: "Equipo 04".to_string(),
            participante_nombre: "Alumno demo".to_string(),
            sesion_id: Uuid::new_v4().to_string()[..8].to_uppercase(),
            fecha_inicio: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),

            tiempo_restante_segundos: 300,
            temporizador_activo: true,
            tiempo_transcurrido_reto: 0,

            sql_buffer: initial_sql,

            query_result: None,
            validation_result: None,
            feedback_banner: Some((
                false,
                "Preparado: Escribe tu consulta SQL y pulsa 'Ejecutar y comprobar'.".to_string(),
            )),

            schema,
            tablas_expandidas,

            retos_superados: Vec::new(),
            intentos_por_reto: HashMap::new(),
            historial_intentos: Vec::new(),
            tiempo_total_segundos: 0,

            mostrar_modal_ranking: false,
            mostrar_pistas: false,
            mensaje_sistema: None,
        }
    }

    pub fn reto_actual(&self) -> Option<&Reto> {
        self.retos.get(self.reto_actual_idx)
    }

    /// Ejecutar y validar la consulta del usuario
    pub fn ejecutar_y_comprobar(&mut self) {
        if self.retos.is_empty() {
            return;
        }
        let reto = self.retos[self.reto_actual_idx].clone();
        let sql = self.sql_buffer.trim().to_string();

        if sql.is_empty() {
            self.feedback_banner = Some((false, "Introduce una sentencia SQL para ejecutar.".to_string()));
            return;
        }

        // 1. Ejecutar consulta para mostrar resultados en tabla (límite 50 filas visibles en UI)
        match self.db.execute_query(&sql, 50) {
            Ok(res) => {
                self.query_result = Some(res);
            }
            Err(err) => {
                self.query_result = None;
                self.feedback_banner = Some((false, format!("Error SQL: {}", err)));

                // Registrar intento fallido por error de sintaxis
                *self.intentos_por_reto.entry(reto.id.clone()).or_insert(0) += 1;
                self.historial_intentos.push(IntentoRegistro {
                    reto_id: reto.id.clone(),
                    sql: sql.clone(),
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    tiempo_segundos: self.tiempo_transcurrido_reto,
                    superado: false,
                    mensaje_error: Some(err),
                });
                return;
            }
        }

        // 2. Validación semántica profunda
        let val_res = Validator::validate(&self.db, &reto, &sql);
        let es_superado = val_res.is_valid;

        *self.intentos_por_reto.entry(reto.id.clone()).or_insert(0) += 1;

        self.historial_intentos.push(IntentoRegistro {
            reto_id: reto.id.clone(),
            sql: sql.clone(),
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            tiempo_segundos: self.tiempo_transcurrido_reto,
            superado: es_superado,
            mensaje_error: if es_superado { None } else { Some(val_res.message.clone()) },
        });

        if es_superado {
            if !self.retos_superados.contains(&reto.id) {
                self.retos_superados.push(reto.id.clone());
            }
            self.feedback_banner = Some((
                true,
                format!("¡Correcta! Consulta validada con éxito: {} filas comprobadas.", val_res.rows_matched),
            ));
        } else {
            self.feedback_banner = Some((false, format!("{}: {}", val_res.title, val_res.message)));
        }

        self.validation_result = Some(val_res);
    }

    /// Avanzar al siguiente reto
    pub fn siguiente_reto(&mut self) {
        if self.reto_actual_idx + 1 < self.retos.len() {
            self.seleccionar_reto(self.reto_actual_idx + 1);
        } else {
            // Fin de la partida: mostrar ranking y exportar
            self.mostrar_modal_ranking = true;
            self.exportar_resultados();
        }
    }

    /// Seleccionar un reto específico de la lista
    pub fn seleccionar_reto(&mut self, idx: usize) {
        if idx < self.retos.len() {
            self.reto_actual_idx = idx;
            let reto = &self.retos[idx];

            self.tiempo_restante_segundos = 300; // Reset a 5 minutos
            self.tiempo_transcurrido_reto = 0;
            self.query_result = None;
            self.validation_result = None;
            self.mostrar_pistas = false;

            self.sql_buffer = format!("-- Reto {}: {}\nSELECT ", reto.id, reto.titulo);
            self.feedback_banner = Some((
                false,
                format!("Reto {} cargado. Tienes 5 minutos para resolverlo.", reto.id),
            ));

            // Expandir la tabla correspondiente en el esquema
            if let Some(expand) = self.tablas_expandidas.get_mut(&reto.tabla) {
                *expand = true;
            }
        }
    }

    /// Tic del temporizador (llamado cada 1 segundo)
    pub fn tick_segundo(&mut self) {
        if self.temporizador_activo && self.tiempo_restante_segundos > 0 {
            self.tiempo_restante_segundos -= 1;
            self.tiempo_transcurrido_reto += 1;
            self.tiempo_total_segundos += 1;

            if self.tiempo_restante_segundos == 0 {
                // Tiempo agotado: registrar y pasar al siguiente
                self.feedback_banner = Some((
                    false,
                    "¡Tiempo agotado (5 min)! Pasando automáticamente al siguiente reto...".to_string(),
                ));
                self.siguiente_reto();
            }
        }
    }

    /// Exportar automáticamente a CSV y JSON como exige la práctica
    pub fn exportar_resultados(&mut self) {
        let resumen = SesionResumen {
            sesion_id: self.sesion_id.clone(),
            equipo_codigo: self.equipo_codigo.clone(),
            fecha_inicio: self.fecha_inicio.clone(),
            fecha_fin: Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
            retos_completados: self.retos_superados.len(),
            total_retos: self.retos.len(),
            total_intentos: self.historial_intentos.len(),
            tiempo_total_segundos: self.tiempo_total_segundos,
            intentos: self.historial_intentos.clone(),
        };

        let json_filename = format!("resultados_{}_{}.json", self.equipo_codigo.replace(' ', "_"), self.sesion_id);
        let csv_filename = format!("resultados_{}_{}.csv", self.equipo_codigo.replace(' ', "_"), self.sesion_id);

        let _ = Exporter::export_json(&resumen, &json_filename);
        let _ = Exporter::export_csv(&resumen, &csv_filename);

        self.mensaje_sistema = Some(format!(
            "Resultados exportados con éxito a '{}' y '{}'.",
            json_filename, csv_filename
        ));
    }

    /// Reiniciar partida limpia
    pub fn reiniciar_partida(&mut self) {
        *self = Self::new();
    }
}
