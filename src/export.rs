use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::models::SesionResumen;

pub struct Exporter;

impl Exporter {
    /// Ruta estándar en Windows: Documentos\BD_Hector_Nerea_Deborah\
    pub fn get_export_dir() -> PathBuf {
        let base_docs = dirs::document_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        base_docs.join("BD_Hector_Nerea_Deborah")
    }

    fn sanitize_filename(name: &str) -> String {
        let cleaned: String = name
            .trim()
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
            .collect();
        if cleaned.is_empty() {
            "alumno".to_string()
        } else {
            cleaned
        }
    }

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

        // Encabezados (sin puntos, centrado en aciertos/errores)
        wtr.write_record(&[
            "SesionID",
            "Participante",
            "Calificacion",
            "RetoID",
            "Timestamp",
            "TiempoEmpleadoSeg",
            "Superado",
            "Error",
            "SQLEjecutado",
        ])
        .map_err(|e| format!("Error escribiendo cabecera CSV: {}", e))?;

        for intento in &sesion.intentos {
            wtr.write_record(&[
                &sesion.sesion_id,
                &sesion.participante,
                &sesion.calificacion,
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

    /// Realizar exportación completa en 'Documentos\BD_Hector_Nerea_Deborah\' y devolver rutas
    pub fn export_all(sesion: &SesionResumen) -> Result<(PathBuf, PathBuf), String> {
        let export_dir = Self::get_export_dir();
        if !export_dir.exists() {
            fs::create_dir_all(&export_dir)
                .map_err(|e| format!("No se pudo crear carpeta '{}': {}", export_dir.display(), e))?;
        }

        let safe_user = Self::sanitize_filename(&sesion.participante);
        let base_name = format!("resultados_{}_{}", safe_user, sesion.sesion_id);

        let json_path = export_dir.join(format!("{}.json", base_name));
        let csv_path = export_dir.join(format!("{}.csv", base_name));

        Self::export_json(sesion, &json_path)?;
        Self::export_csv(sesion, &csv_path)?;

        Ok((json_path, csv_path))
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
