# ⚡ Simulador SQL Sprint Davante (GPUI & Rust Edition)

> **Módulo:** Gestión de Bases de Datos (ASIR) — Tema 7: Realización de Consultas  
> **Centro:** Instituto Davante  
> **Tecnología:** Rust 2021 + GPUI (Acelerado por GPU vía DirectX 12) + SQLite nativo (`rusqlite`)

---

## 📋 Descripción del Proyecto

El **Simulador SQL Sprint Davante** es una aplicación de escritorio nativa de ultra-alto rendimiento diseñada para practicar y validar la construcción de sentencias SQL sobre la base de datos relacional de gestión académica y de recursos del centro formativo (`davante_asir.db`).

A diferencia de soluciones convencionales basadas en Electron o Python/Tkinter, esta versión implementa:
1. **Renderizado acelerado por hardware (GPU):** Utiliza **GPUI** (el motor gráfico desarrollado por el equipo de **Zed**), logrando una tasa de refresco constante de **120+ FPS**, latencia de entrada de sub-milisegundo y un consumo de memoria RAM inferior a **30 MB**.
2. **Consultas a velocidad nativa (C/Rust ABI):** Comunicación directa en memoria con SQLite sin capas HTTP intermedias ni latencia de red, resolviendo consultas indexadas en **microsegundos (0.01 - 0.04 ms)** sobre una base de datos con más de **425.000 registros**.
3. **Seguridad y Aislamiento estricto:** Conexión obligatoria en modo lectura (`mode=ro`) y un **autorizador SQLite (`set_authorizer`)** en el nivel C-ABI que deniega tajantemente cualquier intento de ejecución DDL o DML (`INSERT`, `UPDATE`, `DELETE`, `DROP`, `ALTER`, `ATTACH`, `PRAGMA`).

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

## 🎯 Banco de 30 Retos (Cargado externamente desde `assets/retos.json`)

Los ejercicios cubren progresivamente todo el temario de manipulación de datos:

* **Nivel 1 (Q01 - Q06): Selección y proyección básica:**
  `SELECT *`, columnas concretas, alias con `AS`, eliminación de duplicados con `DISTINCT` y concatenación de cadenas mediante el operador `||`.
* **Nivel 2 (Q07 - Q12): Filtrado condicional con WHERE:**
  Operadores relacionales (`=`, `>`), rangos con `BETWEEN`, listas con `IN`, coincidencia de patrones con `LIKE 'Proyecto%'` y control de nulos con `IS NULL`.
* **Nivel 3 (Q13 - Q18): Ordenación, límites y expresiones:**
  Lógica booleana `AND`, negación (`<>` o `NOT`), ordenaciones compuestas (`ORDER BY fecha DESC, titulo ASC`), limitación de salida (`LIMIT`) y operaciones aritméticas.
* **Nivel 4 (Q19 - Q24): Resumen y funciones de agregado:**
  `COUNT(*)`, `COUNT(columna)` omitiendo nulos, `SUM()`, `AVG()`, `MIN()`, `MAX()`, y agrupamiento simple y multinivel con `GROUP BY`.
* **Nivel 5 (Q25 - Q30): Filtrado posterior a la agregación con HAVING:**
  Uso de `HAVING` sobre agregados, combinación de `WHERE` y `HAVING`, y ordenación de conjuntos agrupados (`LIMIT 5`).

---

## ⚖️ Motor de Validación Semántica

Siguiendo las directrices del documento oficial (página 10):
1. **Validación agnóstica del texto:** No se compara la cadena SQL del alumno con la solución; se ejecuta la consulta y se evalúa el resultado generado frente a la consulta de referencia.
2. **Comparación Multiset:** Para consultas sin requisito de ordenación, las filas se comparan como una bolsa de elementos (*multiset*), garantizando que las consultas sean correctas sin importar el orden no determinista en que SQLite devuelva las filas.
3. **Tolerancia en punto flotante:** Comparación de números reales (`AVG`, notas, pesos) con un delta $\epsilon < 0.01$.
4. **Preservación estricta de `NULL`:** Los valores nulos son tratados independientemente de cadenas vacías o ceros.

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

---

## 🎮 Guía de Uso del Simulador

1. **Panel Izquierdo (Banco de Retos):** Selecciona cualquier reto disponible del catálogo (Q01 a Q30). El icono cambiará a `✓` verde conforme los vayas superando.
2. **Panel Central:**
   * **Temporizador:** Cuenta atrás de 5 minutos por reto (se vuelve ámbar al restar 1 minuto y rojo en los últimos 30 segundos).
   * **Paleta Rápida SQL:** Botones para insertar instantáneamente cláusulas clave (`SELECT`, `WHERE`, `GROUP BY`, etc.).
   * **Editor:** Escribe tu consulta libremente. Pulsa `[▶ Ejecutar y comprobar]` o presiona `Ctrl + Enter`.
   * **Visor de Resultados:** Muestra las filas devueltas, columnas tipadas y el tiempo de respuesta exacto en milisegundos.
3. **Panel Derecho:**
   * **Esquema de BD:** Árbol interactivo para inspeccionar las 12 tablas y sus campos. Haz clic en `+ Insertar` o en el nombre de cualquier columna para añadirla al editor.
   * **Estadísticas:** Control en tiempo real de aciertos, intentos y cronómetro global.
4. **Exportación:**
   * Al finalizar la sesión o pulsando `[💾 Exportar CSV/JSON]`, se generan automáticamente los archivos `resultados_EquipoXX_XXXX.json` y `.csv` con todas las métricas, marcas de tiempo y consultas realizadas para su entrega.
