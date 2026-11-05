use std::fs::File;
use std::io::Write;
use std::path::Path;
use crate::models::SesionResumen;

pub struct Exporter;

impl Exporter {
    /// Export session summary to JSON
    pub fn export_json<P: AsRef<Path>>(sesion: &SesionResumen, path: P) -> Result<(), String> {
        let json_str = serde_json::to_string_pretty(sesion)
            .map_err(|e| format!("Error serializando a JSON: {}", e))?;
        let mut file = File::create(path).map_err(|e| format!("Error creando archivo JSON: {}", e))?;
        file.write_all(json_str.as_bytes())
            .map_err(|e| format!("Error escribiendo JSON: {}", e))?;
        Ok(())
    }

    /// Export session summary to CSV
    pub fn export_csv<P: AsRef<Path>>(sesion: &SesionResumen, path: P) -> Result<(), String> {
        let mut wtr = csv::Writer::from_path(path)
            .map_err(|e| format!("Error creando archivo CSV: {}", e))?;

        // Encabezados
        wtr.write_record(&[
            "SesionID",
            "Equipo",
            "RetoID",
            "Timestamp",
            "TiempoSegundos",
            "Superado",
            "Error",
            "SQLEjecutado",
        ])
        .map_err(|e| format!("Error escribiendo cabecera CSV: {}", e))?;

        for intento in &sesion.intentos {
            wtr.write_record(&[
                &sesion.sesion_id,
                &sesion.equipo_codigo,
                &intento.reto_id,
                &intento.timestamp,
                &intento.tiempo_segundos.to_string(),
                &intento.superado.to_string(),
                intento.mensaje_error.as_deref().unwrap_or(""),
                &intento.sql,
            ])
            .map_err(|e| format!("Error escribiendo fila CSV: {}", e))?;
        }

        wtr.flush().map_err(|e| format!("Error cerrando CSV: {}", e))?;
        Ok(())
    }
}
