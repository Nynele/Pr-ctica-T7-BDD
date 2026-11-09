use std::collections::HashMap;
use std::fs;
use std::path::Path;
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
    pub participante_nombre: String,
    pub sesion_id: String,
    pub fecha_inicio: String,

    // Temporizador ÚNICO GLOBAL (5 minutos = 300 segundos para TODO el sprint)
    pub tiempo_restante_global: u32,
    pub temporizador_activo: bool,

    // Editor SQL
    pub sql_buffer: String,

    // Resultados
    pub query_result: Option<QueryResult>,
    pub validation_result: Option<ValidationResult>,
    pub feedback_banner: Option<(bool, String)>, // (es_exito, mensaje)

    // Esquema de la BD
    pub schema: Vec<SchemaTable>,
    pub tablas_expandidas: HashMap<String, bool>,

    // Categorías de retos expandidas en la barra lateral
    pub categorias_expandidas: HashMap<u8, bool>,

    // Progreso y métricas
    pub retos_superados: Vec<String>, // IDs de retos acertados
    pub intentos_por_reto: HashMap<String, usize>,
    pub historial_intentos: Vec<IntentoRegistro>,
    pub tiempo_total_segundos: u64,

    // Modales y vistas
    pub mostrar_modal_ranking: bool,
    pub mostrar_modal_exportacion: bool,
    pub mostrar_pistas: bool,
    pub ruta_export_json: Option<String>,
    pub ruta_export_csv: Option<String>,
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

        // Expandir todas las categorías de retos por defecto
        let mut categorias_expandidas = HashMap::new();
        for lvl in 1..=5 {
            categorias_expandidas.insert(lvl, true);
        }

        let initial_sql = if !retos.is_empty() {
            format!("-- Reto {}: {}\nSELECT ", retos[0].id, retos[0].titulo)
        } else {
            "SELECT ".to_string()
        };

        Self {
            db,
            retos,
            reto_actual_idx: 0,
            participante_nombre: "Alumno demo".to_string(),
            sesion_id: Uuid::new_v4().to_string()[..8].to_uppercase(),
            fecha_inicio: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),

            // TIEMPO GLOBAL DE 5 MINUTOS (300 SEGUNDOS) PARA TODO EL SIMULADOR
            tiempo_restante_global: 300,
            temporizador_activo: true,

            sql_buffer: initial_sql,

            query_result: None,
            validation_result: None,
            feedback_banner: Some((
                false,
                "Preparado: Tienes 5 minutos globales para completar los retos. ¡Buena suerte!".to_string(),
            )),

            schema,
            tablas_expandidas,
            categorias_expandidas,

            retos_superados: Vec::new(),
            intentos_por_reto: HashMap::new(),
            historial_intentos: Vec::new(),
            tiempo_total_segundos: 0,

            mostrar_modal_ranking: false,
            mostrar_modal_exportacion: false,
            mostrar_pistas: false,
            ruta_export_json: None,
            ruta_export_csv: None,
        }
    }

    pub fn reto_actual(&self) -> Option<&Reto> {
        self.retos.get(self.reto_actual_idx)
    }

    /// Calcula la puntuación total acumulada sumando los puntos de cada reto superado
    pub fn puntos_actuales(&self) -> u32 {
        self.retos
            .iter()
            .filter(|r| self.retos_superados.contains(&r.id))
            .map(|r| r.puntos())
            .sum()
    }

    /// Puntos totales posibles de los 30 retos (N1=6*10, N2=6*20, N3=6*30, N4=6*40, N5=6*50 = 900)
    pub fn puntos_maximos(&self) -> u32 {
        self.retos.iter().map(|r| r.puntos()).sum()
    }

    /// Ejecutar y validar la consulta del usuario
    pub fn ejecutar_y_comprobar(&mut self) {
        if self.retos.is_empty() {
            return;
        }

        // Si el tiempo global se agotó, impedir ejecuciones adicionales
        if self.tiempo_restante_global == 0 {
            self.feedback_banner = Some((
                false,
                "⏱ El tiempo global de 5 minutos ha finalizado. Puedes reiniciar la partida para un nuevo intento.".to_string(),
            ));
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
                    tiempo_segundos: self.tiempo_total_segundos,
                    superado: false,
                    puntos_obtenidos: 0,
                    mensaje_error: Some(err),
                });
                return;
            }
        }

        // 2. Validación semántica profunda
        let val_res = Validator::validate(&self.db, &reto, &sql);
        let es_superado = val_res.is_valid;
        let puntos_ganados = if es_superado { reto.puntos() } else { 0 };

        *self.intentos_por_reto.entry(reto.id.clone()).or_insert(0) += 1;

        self.historial_intentos.push(IntentoRegistro {
            reto_id: reto.id.clone(),
            sql: sql.clone(),
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            tiempo_segundos: self.tiempo_total_segundos,
            superado: es_superado,
            puntos_obtenidos: puntos_ganados,
            mensaje_error: if es_superado { None } else { Some(val_res.message.clone()) },
        });

        if es_superado {
            let ya_estaba_superado = self.retos_superados.contains(&reto.id);
            if !ya_estaba_superado {
                self.retos_superados.push(reto.id.clone());
            }
            self.feedback_banner = Some((
                true,
                format!(
                    "¡Correcta! (+{} pts) Consulta validada con éxito: {} filas comprobadas.",
                    reto.puntos(),
                    val_res.rows_matched
                ),
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
            // Fin de la partida
            self.mostrar_modal_ranking = true;
            self.exportar_resultados();
        }
    }

    /// Seleccionar un reto específico de la lista (EL TIEMPO NO SE REINICIA, ES GLOBAL)
    pub fn seleccionar_reto(&mut self, idx: usize) {
        if idx < self.retos.len() {
            self.reto_actual_idx = idx;
            let reto = &self.retos[idx];

            // NO SE REINICIA EL TIEMPO: el tiempo es global de 5 minutos
            self.query_result = None;
            self.validation_result = None;
            self.mostrar_pistas = false;

            self.sql_buffer = format!("-- Reto {}: {}\nSELECT ", reto.id, reto.titulo);
            self.feedback_banner = Some((
                false,
                format!("Reto {} cargado (Nivel {} · {} · {} pts).", reto.id, reto.nivel, reto.dificultad_label(), reto.puntos()),
            ));

            // Expandir la tabla correspondiente en el esquema
            if let Some(expand) = self.tablas_expandidas.get_mut(&reto.tabla) {
                *expand = true;
            }

            // Asegurar que la categoría del reto actual esté expandida
            self.categorias_expandidas.insert(reto.nivel, true);
        }
    }

    /// Alternar entre expandir y contraer una categoría en la barra lateral
    pub fn toggle_categoria(&mut self, nivel: u8) {
        let current = *self.categorias_expandidas.get(&nivel).unwrap_or(&true);
        self.categorias_expandidas.insert(nivel, !current);
    }

    /// Tic del temporizador global (llamado cada 1 segundo)
    pub fn tick_segundo(&mut self) {
        if self.temporizador_activo && self.tiempo_restante_global > 0 {
            self.tiempo_restante_global -= 1;
            self.tiempo_total_segundos += 1;

            if self.tiempo_restante_global == 0 {
                // TIEMPO GLOBAL DE 5 MINUTOS AGOTADO
                self.temporizador_activo = false;
                self.feedback_banner = Some((
                    false,
                    "⏱ ¡TIEMPO GLOBAL DE 5 MINUTOS AGOTADO! El sprint ha concluido. Consulta tu puntuación y exporta tus resultados.".to_string(),
                ));
                self.mostrar_modal_ranking = true;
                self.exportar_resultados();
            }
        }
    }

    /// Exportar a CSV y JSON de forma 100% funcional y guardar las rutas
    pub fn exportar_resultados(&mut self) {
        let resumen = SesionResumen {
            sesion_id: self.sesion_id.clone(),
            participante: self.participante_nombre.clone(),
            fecha_inicio: self.fecha_inicio.clone(),
            fecha_fin: Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
            retos_completados: self.retos_superados.len(),
            total_retos: self.retos.len(),
            total_intentos: self.historial_intentos.len(),
            puntos_obtenidos: self.puntos_actuales(),
            puntos_totales_posibles: self.puntos_maximos(),
            tiempo_total_segundos: self.tiempo_total_segundos,
            tiempo_restante_global: self.tiempo_restante_global,
            intentos: self.historial_intentos.clone(),
        };

        match Exporter::export_all(&resumen) {
            Ok((json_path, csv_path)) => {
                let json_str = json_path.to_string_lossy().to_string();
                let csv_str = csv_path.to_string_lossy().to_string();
                self.ruta_export_json = Some(json_str);
                self.ruta_export_csv = Some(csv_str);
                self.mostrar_modal_exportacion = true;
            }
            Err(err) => {
                self.feedback_banner = Some((false, format!("Error al exportar: {}", err)));
            }
        }
    }

    /// Abrir la carpeta donde se han exportado los archivos
    pub fn abrir_carpeta_exportacion(&self) {
        Exporter::abrir_carpeta_en_explorador(Path::new("exports"));
    }

    /// Reiniciar partida limpia (con 5 minutos globales nuevos)
    pub fn reiniciar_partida(&mut self) {
        *self = Self::new();
    }
}
