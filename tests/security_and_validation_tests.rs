use simulador_sql_davante::app_state::{SimuladorState, TIEMPO_LIMITE_POR_RETO_SEGS};
use simulador_sql_davante::db::DbManager;
use simulador_sql_davante::export::{Exporter, CSV_FILENAME, JSON_FILENAME};
use simulador_sql_davante::models::{Reto, SesionResumen};
use simulador_sql_davante::validator::Validator;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[test]
fn test_load_all_30_retos() {
    let content = fs::read_to_string("assets/retos.json").expect("assets/retos.json should exist");
    let retos: Vec<Reto> = serde_json::from_str(&content).expect("Invalid JSON in retos.json");
    assert_eq!(retos.len(), 30, "Debe haber exactamente 30 retos configurados");
    assert_eq!(retos[0].id, "Q01");
    assert_eq!(retos[29].id, "Q30");
}

#[test]
fn test_security_authorizer_blocks_dml_and_ddl() {
    let db = DbManager::new("assets/davante_asir.db");

    // Probar INSERT
    let res_insert = db.execute_query("INSERT INTO alumnos (nombre, apellido1, localidad, fecha_alta, activo) VALUES ('Hacker', 'Test', 'Madrid', '2026-01-01', 1);", 10);
    assert!(res_insert.is_err(), "INSERT debe ser bloqueado por el autorizador");

    // Probar DROP
    let res_drop = db.execute_query("DROP TABLE alumnos;", 10);
    assert!(res_drop.is_err(), "DROP TABLE debe ser bloqueado");

    // Probar UPDATE
    let res_update = db.execute_query("UPDATE alumnos SET nombre='Modificado';", 10);
    assert!(res_update.is_err(), "UPDATE debe ser bloqueado");

    // Probar DELETE
    let res_delete = db.execute_query("DELETE FROM alumnos WHERE id_alumno = 1;", 10);
    assert!(res_delete.is_err(), "DELETE debe ser bloqueado");

    // Probar sentencias encadenadas con ';'
    let res_chained = db.execute_query("SELECT * FROM alumnos; DROP TABLE alumnos;", 10);
    assert!(res_chained.is_err(), "Sentencias encadenadas deben ser bloqueadas");

    let res_chained_select = db.execute_query("SELECT 1; SELECT 2;", 10);
    assert!(res_chained_select.is_err(), "Múltiples SELECT separados por ';' deben bloquearse");

    // Probar PRAGMA y ATTACH
    let res_pragma = db.execute_query("PRAGMA table_info(alumnos);", 10);
    assert!(res_pragma.is_err(), "PRAGMA debe ser bloqueado");

    let res_attach = db.execute_query("ATTACH DATABASE 'otra.db' AS otra;", 10);
    assert!(res_attach.is_err(), "ATTACH DATABASE debe ser bloqueado");

    // Probar entradas vacías y solo comentarios
    assert!(db.execute_query("", 10).is_err());
    assert!(db.execute_query("   \n  -- solo comentario ", 10).is_err());

    // Probar que una consulta con comentario inicial -- Reto ... funciona correctamente
    let res_commented = db.execute_query("-- Reto Q01: Comentario\nSELECT * FROM alumnos LIMIT 5;", 10);
    assert!(res_commented.is_ok(), "Una consulta SELECT precedida por comentario '--' debe funcionar");
    assert_eq!(res_commented.unwrap().rows.len(), 5);

    // Probar límite de visualización de 50 filas mientras total_rows refleja el total real
    let res_50 = db.execute_query("SELECT * FROM alumnos;", 50).unwrap();
    assert_eq!(res_50.rows.len(), 50, "La visualización debe limitarse a 50 filas");
    assert!(res_50.total_rows > 50 && res_50.is_truncated, "total_rows e is_truncated deben reflejar la truncación");
}

#[test]
fn test_all_30_canonical_queries_validate_successfully() {
    let db = DbManager::new("assets/davante_asir.db");
    let content = fs::read_to_string("assets/retos.json").expect("retos.json exists");
    let retos: Vec<Reto> = serde_json::from_str(&content).unwrap();

    for reto in retos {
        let val = Validator::validate(&db, &reto, &reto.consulta_referencia);
        assert!(
            val.is_valid,
            "El reto {} falló su propia validación canónica: {} - {}",
            reto.id, val.title, val.message
        );
    }
}

#[test]
fn test_validation_detects_wrong_query() {
    let db = DbManager::new("assets/davante_asir.db");
    let reto = Reto {
        id: "TEST_Q".to_string(),
        nivel: 1,
        titulo: "Test".to_string(),
        peticion: "Solo id y nombre".to_string(),
        tabla: "alumnos".to_string(),
        elementos_sql: "SELECT".to_string(),
        consulta_referencia: "SELECT id_alumno, nombre FROM alumnos LIMIT 5;".to_string(),
        requiere_orden: false,
        pistas: vec![],
    };

    // Consulta con columnas erróneas
    let wrong_cols = Validator::validate(&db, &reto, "SELECT id_alumno FROM alumnos LIMIT 5;");
    assert!(!wrong_cols.is_valid, "Debe detectar número de columnas erróneo");

    // Consulta con número de filas erróneas
    let wrong_rows = Validator::validate(&db, &reto, "SELECT id_alumno, nombre FROM alumnos LIMIT 2;");
    assert!(!wrong_rows.is_valid, "Debe detectar discrepancia en filas");

    // Consulta con orden de columnas invertido
    let wrong_col_order = Validator::validate(&db, &reto, "SELECT nombre, id_alumno FROM alumnos LIMIT 5;");
    assert!(!wrong_col_order.is_valid, "Debe detectar orden de columnas erróneo");

    // Consulta con tipo distinto (texto en lugar de entero)
    let wrong_type = Validator::validate(&db, &reto, "SELECT CAST(id_alumno AS TEXT), nombre FROM alumnos LIMIT 5;");
    assert!(!wrong_type.is_valid, "Debe distinguir Text de Integer en la evaluación de tipos");

    // Consulta equivalente con comentario inicial y mayúsculas/minúsculas mixtas
    let equiv_ok = Validator::validate(&db, &reto, "-- Comentario\nselect id_alumno, nombre from alumnos limit 5");
    assert!(equiv_ok.is_valid, "Debe validar consulta equivalente con comentario inicial y sin ';'");
}

#[test]
fn test_semicolon_inside_literal_allowed() {
    let db = DbManager::new("assets/davante_asir.db");
    let res = db.execute_query("SELECT * FROM alumnos WHERE nombre LIKE '%;%';", 10);
    assert!(res.is_ok(), "Un ';' dentro de comillas simples no es una sentencia encadenada");
}

#[test]
fn test_schema_excludes_actividades_evaluables_and_export_path() {
    let db = DbManager::new("assets/davante_asir.db");
    let schema = db.get_schema().expect("Debe leer el esquema");
    assert!(
        !schema.iter().any(|t| t.name == "actividades_evaluables"),
        "El esquema no debe mostrar actividades_evaluables"
    );
    assert_eq!(schema.len(), 11, "Deben cargarse las 11 tablas restantes");

    let export_dir = Exporter::get_export_dir();
    assert_eq!(
        export_dir.file_name().and_then(|s| s.to_str()),
        Some("BD_Hector_Nerea_Deborah"),
        "La carpeta de exportación debe llamarse BD_Hector_Nerea_Deborah dentro de Documentos"
    );
}

#[test]
fn test_session_lifecycle_timer_and_blocking() {
    let temp = TempDirGuard::new("simulador_sql_lifecycle");
    let mut state = SimuladorState::new_with_export_dir(temp.0.clone());
    assert!(!state.sesion_iniciada, "Debe empezar en la pantalla de inicio");
    assert!(!state.temporizador_activo, "El cronómetro no debe correr antes de comenzar");
    assert_eq!(TIEMPO_LIMITE_POR_RETO_SEGS, 300);
    assert_eq!(state.tiempo_restante_reto_actual(), 300);
    assert!(!state.reto_actual_tiempo_agotado());
    for reto in &state.retos {
        assert_eq!(
            state.tiempo_restante_reto(&reto.id),
            300,
            "Cada reto debe inicializarse con 300s (05:00)"
        );
    }
    assert_eq!(state.calificacion_texto(), "0/30");
    assert!(
        !state.retos.iter().any(|r| r.tabla == "actividades_evaluables"),
        "Ningún reto debe apuntar a actividades_evaluables"
    );

    // Tick antes de comenzar no descuenta tiempo
    state.tick_segundo();
    assert_eq!(state.tiempo_restante_reto_actual(), 300);
    assert_eq!(state.tiempo_total_segundos, 0);

    // Intentar ejecutar antes de iniciar está bloqueado
    state.sql_buffer = state.retos[0].consulta_referencia.clone();
    state.ejecutar_y_comprobar();
    assert_eq!(state.historial_intentos.len(), 0);

    // Iniciar sesión
    state.iniciar_sesion();
    assert!(state.sesion_iniciada);
    assert!(state.temporizador_activo);

    let reto0_id = state.retos[0].id.clone();
    let reto1_id = state.retos[1].id.clone();

    state.tick_segundo();
    assert_eq!(state.tiempo_restante_reto(&reto0_id), 299);
    assert_eq!(state.tiempo_restante_reto(&reto1_id), 300);
    assert_eq!(state.tiempo_total_segundos, 1);

    // Respuesta incorrecta debe activar pistas automáticamente y sumar error
    state.sql_buffer = "SELECT id_alumno FROM alumnos LIMIT 5;".to_string();
    state.ejecutar_y_comprobar();
    assert_eq!(state.errores_cometidos(), 1);
    assert_eq!(state.calificacion_texto(), "0/30");
    assert!(state.mostrar_pistas, "Las pistas deben mostrarse automáticamente al fallar");

    // Consumir los 299 segundos restantes en el primer reto
    for _ in 0..299 {
        state.tick_segundo();
    }
    assert_eq!(state.tiempo_restante_reto(&reto0_id), 0);
    assert!(state.reto_actual_tiempo_agotado());
    assert_eq!(state.tiempo_total_segundos, 300);
    assert!(
        !state.sesion_finalizada,
        "Agotar los 300s de un reto no debe finalizar toda la sesión"
    );
    assert!(state.temporizador_activo);
    assert!(
        matches!(state.status_feedback, Some((false, ref msg)) if msg.contains("Tiempo agotado")),
        "Debe mostrarse aviso claro de tiempo agotado para el reto"
    );

    // Intentar ejecutar la consulta canónica del primer reto con tiempo agotado queda bloqueado
    let intentos_antes_bloqueo = state.historial_intentos.len();
    state.sql_buffer = state.retos[0].consulta_referencia.clone();
    state.ejecutar_y_comprobar();
    assert_eq!(
        state.historial_intentos.len(),
        intentos_antes_bloqueo,
        "No debe ejecutarse ni registrarse intento si el reto tiene el tiempo agotado"
    );
    assert_eq!(state.retos_superados.len(), 0);

    // Cambiar al segundo reto: conserva sus 300s completos
    state.seleccionar_reto(1);
    assert_eq!(state.reto_actual_idx, 1);
    assert_eq!(state.tiempo_restante_reto_actual(), 300);
    assert!(!state.reto_actual_tiempo_agotado());

    // Volver al primer reto conserva su tiempo agotado en 0 (no permite hacer trampas)
    state.seleccionar_reto(0);
    assert_eq!(state.tiempo_restante_reto_actual(), 0);
    assert!(state.reto_actual_tiempo_agotado());

    // Volver al segundo reto y ejecutar su consulta canónica sí funciona
    state.seleccionar_reto(1);
    state.sql_buffer = state.retos[1].consulta_referencia.clone();
    state.ejecutar_y_comprobar();
    assert_eq!(state.retos_superados.len(), 1);
    assert_eq!(state.calificacion_texto(), "1/30");
    assert_eq!(state.retos_no_superados(), 29);

    // Finalizar sesión bloquea nuevas respuestas, detiene el temporizador y permite exportar/cargar ranking
    state.finalizar_sesion(false);
    assert!(state.sesion_finalizada);
    assert!(!state.temporizador_activo);

    let intentos_antes = state.historial_intentos.len();
    state.seleccionar_reto(2);
    state.sql_buffer = state.retos[2].consulta_referencia.clone();
    state.ejecutar_y_comprobar();
    assert_eq!(
        state.historial_intentos.len(),
        intentos_antes,
        "No debe permitir nuevos envíos tras finalizar"
    );
    assert_eq!(state.calificacion_texto(), "1/30");

    let temp = TempDirGuard::new("simulador_sql_finalizar_export");
    let resumen = state.construir_resumen_sesion();
    Exporter::export_all_in_dir(&resumen, &temp.0)
        .expect("La exportación tras finalizar debe funcionar");
    let ranking = Exporter::cargar_ranking_from_dir(&temp.0)
        .expect("El ranking debe cargarse tras finalizar");
    assert!(
        ranking.iter().any(|r| r.sesion_id == resumen.sesion_id),
        "La sesión finalizada debe aparecer en el ranking"
    );
}

struct TempDirGuard(PathBuf);

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("{}_{}", prefix, Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("Debe crear directorio temporal de pruebas");
        Self(dir)
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn sample_session(
    sesion_id: &str,
    participante: &str,
    retos_completados: usize,
    errores_cometidos: usize,
    tiempo_total_segundos: u64,
) -> SesionResumen {
    let total_retos: usize = 30;
    SesionResumen {
        sesion_id: sesion_id.to_string(),
        participante: participante.to_string(),
        fecha_inicio: "2026-11-12 10:00:00".to_string(),
        fecha_fin: Some("2026-11-12 10:04:00".to_string()),
        calificacion: format!("{}/{}", retos_completados, total_retos),
        retos_completados,
        retos_no_superados: total_retos.saturating_sub(retos_completados),
        total_retos,
        total_intentos: retos_completados + errores_cometidos,
        errores_cometidos,
        tiempo_total_segundos,
        tiempo_limite_por_reto_seg: TIEMPO_LIMITE_POR_RETO_SEGS,
        tiempo_restante_por_reto: HashMap::new(),
        intentos: Vec::new(),
    }
}

#[test]
fn test_single_json_and_csv_initialized_clean_and_empty() {
    let temp = TempDirGuard::new("simulador_sql_clean_init");
    let ranking = Exporter::cargar_ranking_from_dir(&temp.0)
        .expect("Debe inicializar JSON y CSV únicos vacíos y limpios");

    let json_path = temp.0.join(JSON_FILENAME);
    let csv_path = temp.0.join(CSV_FILENAME);
    assert!(json_path.exists(), "Debe existir el archivo JSON único");
    assert!(csv_path.exists(), "Debe existir el archivo CSV único");

    let json_content = fs::read_to_string(&json_path).unwrap();
    let sesiones: Vec<SesionResumen> = serde_json::from_str(&json_content).unwrap();
    assert!(sesiones.is_empty(), "La clasificación inicial debe estar vacía sin usuarios de prueba");
    assert!(ranking.is_empty());

    // Verificar que el CSV único tiene cabecera y 0 filas de datos
    let mut rdr = csv::Reader::from_path(&csv_path).unwrap();
    let csv_rows: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
    assert!(csv_rows.is_empty());

    // Si los archivos existen pero están vacíos (0 bytes), también deben inicializarse limpios
    fs::write(&json_path, "").unwrap();
    fs::write(&csv_path, "").unwrap();
    let reinitialized = Exporter::cargar_ranking_from_dir(&temp.0).unwrap();
    assert!(reinitialized.is_empty());
}

#[test]
fn test_cumulative_export_and_no_duplicates_on_same_sesion_id() {
    let temp = TempDirGuard::new("simulador_sql_cumulative");
    let inicial = Exporter::cargar_ranking_from_dir(&temp.0).unwrap();
    let base_count = inicial.len();
    assert_eq!(base_count, 0);

    let s1 = sample_session("SESION01", "Laura Gómez", 25, 2, 180);
    let (json_path, csv_path) = Exporter::export_all_in_dir(&s1, &temp.0).unwrap();
    assert_eq!(json_path, temp.0.join(JSON_FILENAME));
    assert_eq!(csv_path, temp.0.join(CSV_FILENAME));

    let s2 = sample_session("SESION02", "Pablo Ruiz", 28, 1, 210);
    Exporter::export_all_in_dir(&s2, &temp.0).unwrap();

    // Re-exportar SESION01 con datos actualizados: no debe duplicarse
    let s1_actualizada = sample_session("SESION01", "Laura Gómez", 30, 1, 195);
    Exporter::export_all_in_dir(&s1_actualizada, &temp.0).unwrap();

    let json_content = fs::read_to_string(&json_path).unwrap();
    let sesiones: Vec<SesionResumen> = serde_json::from_str(&json_content).unwrap();
    assert_eq!(
        sesiones.len(),
        2,
        "Debe conservar exactamente las 2 partidas reales sin duplicar SESION01"
    );

    let coincidencias_s1: Vec<&SesionResumen> = sesiones
        .iter()
        .filter(|s| s.sesion_id == "SESION01")
        .collect();
    assert_eq!(coincidencias_s1.len(), 1);
    assert_eq!(coincidencias_s1[0].retos_completados, 30);
    assert_eq!(coincidencias_s1[0].calificacion, "30/30");

    let mut rdr = csv::Reader::from_path(&csv_path).unwrap();
    let csv_rows: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
    assert_eq!(csv_rows.len(), base_count + 2);
}

#[test]
fn test_ranking_ordering_by_score_desc_and_time_asc() {
    let temp = TempDirGuard::new("simulador_sql_ranking_order");

    // Crear un JSON controlado para aislar la comprobación de ordenación
    let sesiones = vec![
        sample_session("R_LENTO", "Marta Lenta", 28, 1, 220),
        sample_session("R_RAPIDO", "Jorge Rápido", 28, 4, 140),
        sample_session("R_TOP", "Ana Campeona", 30, 0, 260),
        sample_session("R_EMPATE_ERR_MAS", "Luis Dos", 28, 5, 220),
        sample_session("R_BAJO", "Pedro Base", 15, 0, 90),
    ];
    let json_path = Exporter::json_path_in_dir(&temp.0);
    let csv_path = Exporter::csv_path_in_dir(&temp.0);
    Exporter::export_sessions_json(&sesiones, &json_path).unwrap();
    Exporter::export_sessions_csv(&sesiones, &csv_path).unwrap();

    let ranking = Exporter::cargar_ranking_from_dir(&temp.0).unwrap();
    assert_eq!(ranking.len(), 5);

    let orden_ids: Vec<&str> = ranking.iter().map(|r| r.sesion_id.as_str()).collect();
    assert_eq!(
        orden_ids,
        vec!["R_TOP", "R_RAPIDO", "R_LENTO", "R_EMPATE_ERR_MAS", "R_BAJO"]
    );
    assert_eq!(ranking[0].posicion, 1);
    assert_eq!(ranking[1].posicion, 2);
    assert_eq!(ranking[4].posicion, 5);
}

#[test]
fn test_legacy_json_with_tiempo_restante_global_deserializes() {
    let legacy_json = r#"[{
        "sesion_id": "LEGACY01",
        "participante": "Alumno Previo",
        "fecha_inicio": "2026-11-10 10:00:00",
        "fecha_fin": "2026-11-10 10:04:15",
        "calificacion": "25/30",
        "retos_completados": 25,
        "retos_no_superados": 5,
        "total_retos": 30,
        "total_intentos": 28,
        "errores_cometidos": 3,
        "tiempo_total_segundos": 255,
        "tiempo_restante_global": 45,
        "intentos": []
    }]"#;

    let sesiones: Vec<SesionResumen> =
        serde_json::from_str(legacy_json).expect("Debe deserializar JSON previo con tiempo_restante_global");
    assert_eq!(sesiones.len(), 1);
    assert_eq!(sesiones[0].sesion_id, "LEGACY01");
    assert_eq!(sesiones[0].tiempo_limite_por_reto_seg, 45);
    assert!(sesiones[0].tiempo_restante_por_reto.is_empty());
}
