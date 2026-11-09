use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
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
            "Participante",
            "RetoID",
            "Timestamp",
            "TiempoEmpleadoSeg",
            "Superado",
            "PuntosObtenidos",
            "Error",
            "SQLEjecutado",
        ])
        .map_err(|e| format!("Error escribiendo cabecera CSV: {}", e))?;

        for intento in &sesion.intentos {
            wtr.write_record(&[
                &sesion.sesion_id,
                &sesion.participante,
                &intento.reto_id,
                &intento.timestamp,
                &intento.tiempo_segundos.to_string(),
                &intento.superado.to_string(),
                &intento.puntos_obtenidos.to_string(),
                intento.mensaje_error.as_deref().unwrap_or(""),
                &intento.sql,
            ])
            .map_err(|e| format!("Error escribiendo fila CSV: {}", e))?;
        }

        wtr.flush().map_err(|e| format!("Error cerrando CSV: {}", e))?;
        Ok(())
    }

    /// Realizar exportación completa en carpeta 'exports/' y devolver rutas absolutas
    pub fn export_all(sesion: &SesionResumen) -> Result<(PathBuf, PathBuf), String> {
        let export_dir = Path::new("exports");
        if !export_dir.exists() {
            fs::create_dir_all(export_dir).map_err(|e| format!("No se pudo crear carpeta exports: {}", e))?;
        }

        let json_path = export_dir.join("resultados_sprint_sql.json");
        let csv_path = export_dir.join("resultados_sprint_sql.csv");

        Self::export_json(sesion, &json_path)?;
        Self::export_csv(sesion, &csv_path)?;

        let abs_json = fs::canonicalize(&json_path).unwrap_or(json_path);
        let abs_csv = fs::canonicalize(&csv_path).unwrap_or(csv_path);

        Ok((abs_json, abs_csv))
    }

    /// Abrir la carpeta de exportaciones en el explorador de Windows
    pub fn abrir_carpeta_en_explorador(path: &Path) {
        let target = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };

        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer")
                .arg(target)
                .spawn();
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(target)
                .spawn();
        }
    }
}
