# ⚡ Simulador SQL Sprint Davante (GPUI & Rust Edition)

> **Módulo:** Gestión de Bases de Datos (ASIR) — Tema 7: Realización de Consultas  
> **Centro:** Instituto Davante  
> **Tecnología:** Rust 2021 + GPUI (Acelerado por GPU vía DirectX 12) + SQLite nativo (`rusqlite`)

---

## 📋 Novedades y Actualizaciones Recientes

1. **Logotipo Institucional:** Integrado de forma nativa en la cabecera de la aplicación desde `logotipo.png`.
2. **Puntuación y Categorías por Dificultad (900 Puntos Totales):**
   * **Nivel 1 (10 pts/reto · Verde Esmeralda):** Selección y Proyección básica.
   * **Nivel 2 (20 pts/reto · Cian/Azul):** Filtrado condicional con `WHERE`.
   * **Nivel 3 (30 pts/reto · Violeta):** Ordenación múltiple y límites con `ORDER BY` y `LIMIT`.
   * **Nivel 4 (40 pts/reto · Ámbar/Naranja):** Resumen de datos y agregaciones (`COUNT`, `AVG`, `SUM`, `GROUP BY`).
   * **Nivel 5 (50 pts/reto · Rojo Coral):** Agrupación avanzada y filtrado de grupos (`HAVING`).
3. **Temporizador Global Único (5 Minutos):**
   * Un único cronómetro de 5 minutos (300 segundos) para resolver todos los retos del sprint.
   * No se reinicia al cambiar de ejercicio.
4. **Enfoque Individual:** Eliminada la división por equipos para permitir evaluaciones individuales directas del alumno.
5. **Exportación 100% Funcional:**
   * Generación inmediata de `exports/resultados_sprint_sql.json` y `exports/resultados_sprint_sql.csv`.
   * Modal interactivo que muestra las rutas de los archivos exportados y botón directo para abrir la carpeta en el Explorador de Windows.

---

## 🗄️ Estructura de la Base de Datos (`davante_asir.db`)

La base de datos contiene **12 tablas relacionales** con más de **425.000 registros** sintéticos:

| Tabla | Registros | Descripción | Clave Primaria / Índices Relevantes |
| :--- | :---: | :--- | :--- |
| `alumnos` | 5.000 | Datos académicos del alumnado | `id_alumno` (PK) |
| `docentes` | 1.200 | Profesorado, especialidades y sedes | `id_docente` (PK) |
| `grupos` | 1.200 | Grupos, modalidades, turnos y tutores | `id_grupo` (PK), `codigo` (UNIQUE) |
| `ofertas_formativas`| 1.500 | Ciclos y módulos ofertados | `id_oferta` (PK) |
| `matriculas` | 15.000 | Relación alumno-grupo y estado | `id_matricula` (PK), `idx_matriculas_alumno` |
| `sesiones_clase` | 50.000 | Clases programadas por aula y docente | `id_sesion` (PK), `idx_sesiones_grupo_fecha` |
| `asistencias` | 200.000 | Registro de presencia, retraso y faltas | `id_asistencia` (PK), `idx_asistencias_alumno` |
| `actividades_evaluables` | 10.000 | Exámenes, proyectos y prácticas con peso | `id_actividad` (PK), `idx_actividades_oferta` |
| `calificaciones` | 100.000 | Notas finales y convocatorias | `id_calificacion` (PK), `idx_calificaciones_alumno` |
| `espacios` | 1.200 | Aulas, talleres y capacidades | `id_espacio` (PK) |
| `reservas_espacio` | 25.000 | Ocupación de dependencias por docentes | `id_reserva` (PK), `idx_reservas_espacio` |
| `prestamos_equipos`| 15.000 | Cesión de portátiles y periféricos | `id_prestamo` (PK), `idx_prestamos_alumno` |

---

## 🚀 Requisitos e Instalación

### Prerrequisitos
* Sistema Operativo: Windows 10/11 (x64)
* [Rust Toolchain](https://www.rust-lang.org/) (versión 1.80 o superior)
* Microsoft C++ Build Tools (incluidas en Visual Studio Build Tools con SDK de Windows)

### Ejecución de la aplicación

```bash
# 1. Clonar o situarse en el directorio del proyecto
cd "K:\PROGRAMACION\WEBS - IA\github repo\Práctica T7 BDD"

# 2. Ejecutar los tests de seguridad y validación
cargo test

# 3. Compilar y lanzar el simulador acelerado por GPU
cargo run --release
```
