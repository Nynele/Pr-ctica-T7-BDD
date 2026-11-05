use simulador_sql_davante::db::DbManager;
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
}
