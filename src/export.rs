use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::models::{RankingEntry, SesionResumen};

pub const JSON_FILENAME: &str = "resultados_partidas.json";
pub const CSV_FILENAME: &str = "resultados_partidas.csv";

pub struct Exporter;

impl Exporter {
    /// Ruta estándar en Windows: Documentos\BD_Hector_Nerea_Deborah\
    pub fn get_export_dir() -> PathBuf {
        let base_docs = dirs::document_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        base_docs.join("BD_Hector_Nerea_Deborah")
    }

    /// Ruta del archivo JSON único dentro de un directorio dado
    pub fn json_path_in_dir(dir: &Path) -> PathBuf {
        dir.join(JSON_FILENAME)
    }

    /// Ruta del archivo CSV único dentro de un directorio dado
    pub fn csv_path_in_dir(dir: &Path) -> PathBuf {
        dir.join(CSV_FILENAME)
    }

    /// Guardar la lista completa de partidas en el JSON único
    pub fn export_sessions_json<P: AsRef<Path>>(
        sesiones: &[SesionResumen],
        path: P,
    ) -> Result<(), String> {
        let json_str = serde_json::to_string_pretty(sesiones)
            .map_err(|e| format!("Error serializando partidas a JSON: {}", e))?;
        let mut file =
            File::create(path).map_err(|e| format!("Error creando archivo JSON: {}", e))?;
        file.write_all(json_str.as_bytes())
            .map_err(|e| format!("Error escribiendo JSON: {}", e))?;
        Ok(())
    }

    /// Guardar la lista completa de partidas en el CSV único (una fila resumen por partida)
    pub fn export_sessions_csv<P: AsRef<Path>>(
        sesiones: &[SesionResumen],
        path: P,
    ) -> Result<(), String> {
        let mut wtr = csv::Writer::from_path(path)
            .map_err(|e| format!("Error creando archivo CSV: {}", e))?;

        wtr.write_record(&[
            "SesionID",
            "Participante",
            "FechaInicio",
            "FechaFin",
            "Calificacion",
            "RetosCompletados",
            "TotalRetos",
            "TotalIntentos",
            "ErroresCometidos",
            "TiempoTotalSegundos",
        ])
        .map_err(|e| format!("Error escribiendo cabecera CSV: {}", e))?;

        for sesion in sesiones {
            wtr.write_record(&[
                &sesion.sesion_id,
                &sesion.participante,
                &sesion.fecha_inicio,
                sesion.fecha_fin.as_deref().unwrap_or(""),
                &sesion.calificacion,
                &sesion.retos_completados.to_string(),
                &sesion.total_retos.to_string(),
                &sesion.total_intentos.to_string(),
                &sesion.errores_cometidos.to_string(),
                &sesion.tiempo_total_segundos.to_string(),
            ])
            .map_err(|e| format!("Error escribiendo fila CSV: {}", e))?;
        }

        wtr.flush()
            .map_err(|e| format!("Error cerrando CSV: {}", e))?;
        Ok(())
    }

    /// Asegura que los archivos únicos existen en `dir` (inicializándolos vacíos y limpios si faltan)
    pub fn ensure_initialized_in_dir(dir: &Path) -> Result<Vec<SesionResumen>, String> {
        if !dir.exists() {
            fs::create_dir_all(dir)
                .map_err(|e| format!("No se pudo crear carpeta '{}': {}", dir.display(), e))?;
        }

        let json_path = Self::json_path_in_dir(dir);
        let csv_path = Self::csv_path_in_dir(dir);

        let mut needs_json_write = false;
        let mut needs_csv_write = !csv_path.exists()
            || fs::metadata(&csv_path).map(|m| m.len() == 0).unwrap_or(true);

        let sesiones = if json_path.exists() {
            let content = fs::read_to_string(&json_path)
                .map_err(|e| format!("Error leyendo JSON '{}': {}", json_path.display(), e))?;
            if content.trim().is_empty() {
                needs_json_write = true;
                needs_csv_write = true;
                Vec::new()
            } else {
                serde_json::from_str(&content)
                    .map_err(|e| format!("Error parseando JSON '{}': {}", json_path.display(), e))?
            }
        } else {
            needs_json_write = true;
            needs_csv_write = true;
            Vec::new()
        };

        if needs_json_write {
            Self::export_sessions_json(&sesiones, &json_path)?;
        }
        if needs_csv_write {
            Self::export_sessions_csv(&sesiones, &csv_path)?;
        }

        Ok(sesiones)
    }

    /// Añadir o actualizar una partida (por `sesion_id`) en los archivos únicos dentro de `dir`
    pub fn export_all_in_dir(
        sesion: &SesionResumen,
        dir: &Path,
    ) -> Result<(PathBuf, PathBuf), String> {
        let mut sesiones = Self::ensure_initialized_in_dir(dir)?;

        if let Some(existing) = sesiones
            .iter_mut()
            .find(|s| s.sesion_id == sesion.sesion_id)
        {
            *existing = sesion.clone();
        } else {
            sesiones.push(sesion.clone());
        }

        let json_path = Self::json_path_in_dir(dir);
        let csv_path = Self::csv_path_in_dir(dir);

        Self::export_sessions_json(&sesiones, &json_path)?;
        Self::export_sessions_csv(&sesiones, &csv_path)?;

        Ok((json_path, csv_path))
    }

    /// Realizar exportación acumulativa en 'Documentos\BD_Hector_Nerea_Deborah\' y devolver rutas únicas
    pub fn export_all(sesion: &SesionResumen) -> Result<(PathBuf, PathBuf), String> {
        let export_dir = Self::get_export_dir();
        Self::export_all_in_dir(sesion, &export_dir)
    }

    /// Cargar y ordenar el ranking de participantes desde un directorio dado
    pub fn cargar_ranking_from_dir(dir: &Path) -> Result<Vec<RankingEntry>, String> {
        let sesiones = Self::ensure_initialized_in_dir(dir)?;

        let mut entries: Vec<RankingEntry> = sesiones
            .into_iter()
            .map(|s| RankingEntry {
                posicion: 0,
                sesion_id: s.sesion_id,
                participante: s.participante,
                calificacion: s.calificacion,
                retos_completados: s.retos_completados,
                total_retos: s.total_retos,
                errores_cometidos: s.errores_cometidos,
                tiempo_total_segundos: s.tiempo_total_segundos,
                fecha: s.fecha_fin.unwrap_or(s.fecha_inicio),
            })
            .collect();

        entries.sort_by(|a, b| {
            b.retos_completados
                .cmp(&a.retos_completados)
                .then_with(|| a.tiempo_total_segundos.cmp(&b.tiempo_total_segundos))
                .then_with(|| a.errores_cometidos.cmp(&b.errores_cometidos))
                .then_with(|| a.participante.cmp(&b.participante))
                .then_with(|| a.sesion_id.cmp(&b.sesion_id))
        });

        for (idx, entry) in entries.iter_mut().enumerate() {
            entry.posicion = idx + 1;
        }

        Ok(entries)
    }

    /// Cargar y ordenar el ranking de participantes desde la carpeta oficial de exportación
    pub fn cargar_ranking() -> Result<Vec<RankingEntry>, String> {
        let export_dir = Self::get_export_dir();
        Self::cargar_ranking_from_dir(&export_dir)
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
