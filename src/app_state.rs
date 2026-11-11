use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use crate::db::DbManager;
use crate::export::Exporter;
use crate::models::{
    IntentoRegistro, QueryResult, Reto, SchemaTable, SesionResumen, ValidationResult,
};
use crate::validator::Validator;

const EMBEDDED_RETOS: &str = include_str!("../assets/retos.json");

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

    // Resultados y evaluación (sin banner inferior separado)
    pub query_result: Option<QueryResult>,
    pub validation_result: Option<ValidationResult>,
    pub status_feedback: Option<(bool, String)>, // (es_exito, mensaje) mostrado dentro del panel de resultados

    // Esquema de la BD
    pub schema: Vec<SchemaTable>,
    pub tablas_expandidas: HashMap<String, bool>,

    // Categorías de retos expandidas en la barra lateral
    pub categorias_expandidas: HashMap<u8, bool>,

    // Progreso y métricas (evaluación sobre 30 retos, ej. 28/30)
    pub retos_superados: Vec<String>,
    pub intentos_por_reto: HashMap<String, usize>,
    pub historial_intentos: Vec<IntentoRegistro>,
    pub tiempo_total_segundos: u64,

    // Modales y vistas / ciclo de vida de la sesión
    pub sesion_iniciada: bool,
    pub sesion_finalizada: bool,
    pub editando_nombre: bool,
    pub mostrar_modal_ranking: bool,
    pub mostrar_modal_exportacion: bool,
    pub mostrar_pistas: bool,
    pub ruta_export_json: Option<String>,
    pub ruta_export_csv: Option<String>,
}

impl SimuladorState {
    fn load_retos() -> Vec<Reto> {
        let local = PathBuf::from("assets/retos.json");
        if local.exists() {
            if let Ok(content) = fs::read_to_string(local) {
                if let Ok(parsed) = serde_json::from_str::<Vec<Reto>>(&content) {
                    return parsed;
                }
            }
        }
        serde_json::from_str(EMBEDDED_RETOS).unwrap_or_default()
    }

    pub fn new() -> Self {
        let db_path = DbManager::resolve_portable_db();
        let db = DbManager::new(db_path);

        // Cargar 30 retos (archivo local o embebido dentro del .exe)
        let retos = Self::load_retos();

        // Cargar esquema de la BD (sin actividades_evaluables)
        let schema = db.get_schema().unwrap_or_default();
        let mut tablas_expandidas = HashMap::new();
        for t in &schema {
            tablas_expandidas.insert(t.name.clone(), false);
        }
        if !retos.is_empty() && tablas_expandidas.contains_key(&retos[0].tabla) {
            tablas_expandidas.insert(retos[0].tabla.clone(), true);
        }

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

            // TIEMPO GLOBAL DE 5 MINUTOS (300 SEGUNDOS) - en pausa hasta pulsar "Comenzar"
            tiempo_restante_global: 300,
            temporizador_activo: false,

            sql_buffer: initial_sql,

            query_result: None,
            validation_result: None,
            status_feedback: None,

            schema,
            tablas_expandidas,
            categorias_expandidas,

            retos_superados: Vec::new(),
            intentos_por_reto: HashMap::new(),
            historial_intentos: Vec::new(),
            tiempo_total_segundos: 0,

            sesion_iniciada: false,
            sesion_finalizada: false,
            editando_nombre: false,
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

    /// Calificación en formato X/30 (ej. "28/30")
    pub fn calificacion_texto(&self) -> String {
        format!("{}/{}", self.retos_superados.len(), self.retos.len())
    }

    /// Iniciar la sesión desde la pantalla de bienvenida
    pub fn iniciar_sesion(&mut self) {
        self.sesion_iniciada = true;
        self.sesion_finalizada = false;
        self.temporizador_activo = true;
        self.editando_nombre = false;
        self.fecha_inicio = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    }

    /// Finalizar la sesión (por tiempo agotado o por decisión del alumno) y bloquear nuevas respuestas
    pub fn finalizar_sesion(&mut self, _por_tiempo: bool) {
        self.sesion_finalizada = true;
        self.temporizador_activo = false;
        self.mostrar_modal_ranking = false;
    }

    /// Número total de intentos erróneos acumulados en la sesión
    pub fn errores_cometidos(&self) -> usize {
        self.historial_intentos.iter().filter(|i| !i.superado).count()
    }

    /// Número de retos no superados (incorrectos o sin completar)
    pub fn retos_no_superados(&self) -> usize {
        self.retos.len().saturating_sub(self.retos_superados.len())
    }

    /// Ejecutar y validar la consulta del usuario
    pub fn ejecutar_y_comprobar(&mut self) {
        if self.retos.is_empty() || !self.sesion_iniciada {
            return;
        }

        // Bloquear nuevas respuestas si la sesión ya finalizó o el tiempo se agotó
        if self.sesion_finalizada || self.tiempo_restante_global == 0 {
            self.finalizar_sesion(self.tiempo_restante_global == 0);
            return;
        }

        let reto = self.retos[self.reto_actual_idx].clone();
        let sql = self.sql_buffer.trim().to_string();

        if sql.is_empty() {
            self.status_feedback = Some((false, "Introduce una sentencia SQL para ejecutar.".to_string()));
            return;
        }

        // 1. Ejecutar consulta para mostrar resultados en tabla (límite 50 filas visibles en UI)
        match self.db.execute_query(&sql, 50) {
            Ok(res) => {
                self.query_result = Some(res);
            }
            Err(err) => {
                self.query_result = None;
                self.mostrar_pistas = true;
                self.status_feedback = Some((false, format!("Error SQL: {}", err)));

                *self.intentos_por_reto.entry(reto.id.clone()).or_insert(0) += 1;
                self.historial_intentos.push(IntentoRegistro {
                    reto_id: reto.id.clone(),
                    sql: sql.clone(),
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    tiempo_segundos: self.tiempo_total_segundos,
                    superado: false,
                    mensaje_error: Some(err),
                });
                return;
            }
        }

        // 2. Validación semántica profunda (columnas, filas, tipos y valores)
        let val_res = Validator::validate(&self.db, &reto, &sql);
        let es_superado = val_res.is_valid;

        *self.intentos_por_reto.entry(reto.id.clone()).or_insert(0) += 1;

        self.historial_intentos.push(IntentoRegistro {
            reto_id: reto.id.clone(),
            sql: sql.clone(),
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            tiempo_segundos: self.tiempo_total_segundos,
            superado: es_superado,
            mensaje_error: if es_superado { None } else { Some(val_res.message.clone()) },
        });

        if es_superado {
            if !self.retos_superados.contains(&reto.id) {
                self.retos_superados.push(reto.id.clone());
            }
            self.status_feedback = Some((
                true,
                format!("✅ ¡Consulta Correcta! {} filas comprobadas.", val_res.rows_matched),
            ));
        } else {
            self.mostrar_pistas = true;
            self.status_feedback = Some((false, format!("❌ {}: {}", val_res.title, val_res.message)));
        }

        self.validation_result = Some(val_res);
    }

    /// Avanzar al siguiente reto
    pub fn siguiente_reto(&mut self) {
        if self.sesion_finalizada {
            return;
        }
        if self.reto_actual_idx + 1 < self.retos.len() {
            self.seleccionar_reto(self.reto_actual_idx + 1);
        } else {
            self.finalizar_sesion(false);
        }
    }

    /// Seleccionar un reto específico de la lista (EL TIEMPO NO SE REINICIA, ES GLOBAL)
    pub fn seleccionar_reto(&mut self, idx: usize) {
        if self.sesion_finalizada {
            return;
        }
        if idx < self.retos.len() {
            self.reto_actual_idx = idx;
            let reto = &self.retos[idx];

            self.query_result = None;
            self.validation_result = None;
            self.status_feedback = None;
            self.mostrar_pistas = false;

            self.sql_buffer = format!("-- Reto {}: {}\nSELECT ", reto.id, reto.titulo);

            // Expandir únicamente la tabla correspondiente en el esquema si está disponible
            for val in self.tablas_expandidas.values_mut() {
                *val = false;
            }
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
        if self.sesion_iniciada
            && !self.sesion_finalizada
            && self.temporizador_activo
            && self.tiempo_restante_global > 0
        {
            self.tiempo_restante_global -= 1;
            self.tiempo_total_segundos += 1;

            if self.tiempo_restante_global == 0 {
                self.finalizar_sesion(true);
            }
        }
    }

    /// Exportar a CSV y JSON dentro de Documentos\BD_Hector_Nerea_Deborah\
    pub fn exportar_resultados(&mut self) {
        let resumen = SesionResumen {
            sesion_id: self.sesion_id.clone(),
            participante: self.participante_nombre.clone(),
            fecha_inicio: self.fecha_inicio.clone(),
            fecha_fin: Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
            calificacion: self.calificacion_texto(),
            retos_completados: self.retos_superados.len(),
            retos_no_superados: self.retos_no_superados(),
            total_retos: self.retos.len(),
            total_intentos: self.historial_intentos.len(),
            errores_cometidos: self.errores_cometidos(),
            tiempo_total_segundos: self.tiempo_total_segundos,
            tiempo_restante_global: self.tiempo_restante_global,
            intentos: self.historial_intentos.clone(),
        };

        match Exporter::export_all(&resumen) {
            Ok((json_path, csv_path)) => {
                self.ruta_export_json = Some(json_path.to_string_lossy().to_string());
                self.ruta_export_csv = Some(csv_path.to_string_lossy().to_string());
                self.mostrar_modal_exportacion = true;
            }
            Err(err) => {
                self.status_feedback = Some((false, format!("Error al exportar: {}", err)));
            }
        }
    }

    /// Abrir Documentos\BD_Hector_Nerea_Deborah\ en el Explorador de Windows
    pub fn abrir_carpeta_exportacion(&self) {
        let dir = Exporter::get_export_dir();
        let _ = fs::create_dir_all(&dir);
        Exporter::abrir_carpeta_en_explorador(&dir);
    }

    /// Reiniciar partida limpia (conservando el nombre del alumno y volviendo a pantalla de inicio)
    pub fn reiniciar_partida(&mut self) {
        let nombre_actual = self.participante_nombre.clone();
        *self = Self::new();
        self.participante_nombre = nombre_actual;
    }
}
