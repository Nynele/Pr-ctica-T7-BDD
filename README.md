# ⚡ Simulador SQL Sprint Davante (GPUI & Rust Edition)

> **Módulo:** Gestión de Bases de Datos (ASIR) — Tema 7: Realización de Consultas  
> **Centro:** Instituto Davante  
> **Tecnología:** Rust 2021 + GPUI (Acelerado por GPU vía DirectX 12) + SQLite nativo (`rusqlite`)

---

## 📋 Características Principales

1. **Binario `.exe` Portable y Autónomo:**
   * Tanto el logotipo (`logotipo.png`), el banco de retos (`assets/retos.json`) como la base de datos (`davante_asir.db`) quedan embebidos en el ejecutable final, permitiendo compartir únicamente el archivo `.exe`.
2. **Evaluación Directa sobre 30 Retos y Categorías Colapsables (Sin Puntos):**
   * Los 30 retos se agrupan en 5 categorías temáticas extensibles/contraíbles en la barra lateral (`X/6` por bloque) y evaluación global directa sobre 30 (`X/30`, p. ej. `28/30`).
   * **Categoría 1 (Verde Esmeralda):** Selección y Proyección básica.
   * **Categoría 2 (Cian/Azul):** Filtrado condicional con `WHERE`.
   * **Categoría 3 (Violeta):** Ordenación múltiple y límites con `ORDER BY` y `LIMIT`.
   * **Categoría 4 (Ámbar/Naranja):** Resumen de datos y agregaciones (`COUNT`, `AVG`, `SUM`, `GROUP BY`).
   * **Categoría 5 (Rojo Coral):** Agrupación avanzada y filtrado de grupos (`HAVING`).
3. **Temporizador Global Único (5 Minutos) y Control de Sesión:**
   * Pantalla de inicio con instrucciones básicas antes de iniciar la cuenta atrás de 300 segundos.
   * Bloqueo automático e informe final al agotarse el tiempo o pulsar Finalizar.
4. **Enfoque Individual:** Evaluación directa del alumno con nombre editable al inicio.
5. **Exportación en `Documentos\BD_Hector_Nerea_Deborah\`:**
   * Generación automática de informes en JSON y CSV dentro de `Documentos\BD_Hector_Nerea_Deborah\` con botón para abrir la carpeta en el Explorador de Windows.

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
