use simulador_sql_davante::app_state::SimuladorState;
use simulador_sql_davante::db::DbManager;
use simulador_sql_davante::export::Exporter;
use simulador_sql_davante::models::Reto;
use simulador_sql_davante::validator::Validator;
use std::fs;

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
    let mut state = SimuladorState::new();
    assert!(!state.sesion_iniciada, "Debe empezar en la pantalla de inicio");
    assert!(!state.temporizador_activo, "El cronómetro no debe correr antes de comenzar");
    assert_eq!(state.tiempo_restante_global, 300);
    assert_eq!(state.calificacion_texto(), "0/30");
    assert!(
        !state.retos.iter().any(|r| r.tabla == "actividades_evaluables"),
        "Ningún reto debe apuntar a actividades_evaluables"
    );

    // Tick antes de comenzar no descuenta tiempo
    state.tick_segundo();
    assert_eq!(state.tiempo_restante_global, 300);

    // Intentar ejecutar antes de iniciar está bloqueado
    state.sql_buffer = "SELECT * FROM alumnos LIMIT 20;".to_string();
    state.ejecutar_y_comprobar();
    assert_eq!(state.historial_intentos.len(), 0);

    // Iniciar sesión
    state.iniciar_sesion();
    assert!(state.sesion_iniciada);
    assert!(state.temporizador_activo);

    state.tick_segundo();
    assert_eq!(state.tiempo_restante_global, 299);
    assert_eq!(state.tiempo_total_segundos, 1);

    // Respuesta incorrecta debe activar pistas automáticamente y sumar error
    state.sql_buffer = "SELECT id_alumno FROM alumnos LIMIT 5;".to_string();
    state.ejecutar_y_comprobar();
    assert_eq!(state.errores_cometidos(), 1);
    assert_eq!(state.calificacion_texto(), "0/30");
    assert!(state.mostrar_pistas, "Las pistas deben mostrarse automáticamente al fallar");

    // Respuesta correcta suma acierto sobre 30
    state.sql_buffer = "-- Reto Q01\nSELECT * FROM alumnos LIMIT 20;".to_string();
    state.ejecutar_y_comprobar();
    assert_eq!(state.retos_superados.len(), 1);
    assert_eq!(state.calificacion_texto(), "1/30");
    assert_eq!(state.retos_no_superados(), 29);

    // Finalizar sesión bloquea nuevas respuestas y detiene el temporizador
    state.finalizar_sesion(false);
    assert!(state.sesion_finalizada);
    assert!(!state.temporizador_activo);

    let intentos_antes = state.historial_intentos.len();
    state.seleccionar_reto(1);
    state.sql_buffer = "SELECT id_alumno, nombre, apellido1, localidad, fecha_alta FROM alumnos;".to_string();
    state.ejecutar_y_comprobar();
    assert_eq!(state.historial_intentos.len(), intentos_antes, "No debe permitir nuevos envíos tras finalizar");
    assert_eq!(state.calificacion_texto(), "1/30");
}
