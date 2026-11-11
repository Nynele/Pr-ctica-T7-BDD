# MEMORIA TÉCNICA DEL PROYECTO: SIMULADOR SQL SPRINT DAVANTE

**Ciclo Formativo:** Administración de Sistemas Informáticos en Red (ASIR)  
**Módulo:** Gestión de Bases de Datos  
**Tema:** Tema 7 — Realización de Consultas  
**Entidad / Centro:** Instituto Davante  
**Alumno / Desarrollador:** Alumno ASIR  

---

## 1. INTRODUCCIÓN Y OBJETIVOS

El presente proyecto consiste en el diseño, desarrollo e implementación de un **simulador interactivo de consultas SQL** ambientado en el entorno de gestión académica del **Instituto Davante**. 

### 1.1. Objetivos del Proyecto
* **Consulta y manipulación de datos:** Practicar la construcción de sentencias SQL mediante la cláusula `SELECT`, abarcando filtrado (`WHERE`), tratamiento de nulos (`IS NULL`), ordenaciones compuestas (`ORDER BY`), cálculos aritméticos, agregaciones (`COUNT`, `AVG`, `SUM`, `MIN`, `MAX`) y agrupaciones avanzadas (`GROUP BY` y `HAVING`).
* **Seguridad y fiabilidad:** Garantizar la inmutabilidad de la base de datos mediante conexiones en modo solo lectura (`mode=ro`), autorizadores a nivel de driver SQLite y bloqueo de inyecciones o sentencias encadenadas.
* **Validación semántica:** Comprobar la corrección de las consultas comparando el resultado devuelto (columnas, filas, tipos y valores) frente a consultas canónicas de referencia, aceptando sintaxis equivalentes.
* **Puntuación por niveles y tiempo global:** Sistema de evaluación cronometrado a 5 minutos globales con progresión de puntos (10 a 50 pts por ejercicio) y un máximo de 900 puntos.
* **Alto rendimiento e innovación tecnológica:** Implementar la aplicación en **Rust** con el framework de interfaz gráfica **GPUI**, aprovechando la aceleración directa por GPU y el acceso nativo a memoria para procesar bases de datos de cientos de miles de registros a velocidad instantánea.

---

## 2. DISEÑO DEL SIMULADOR

El simulador se diseña como una herramienta interactiva para practicar consultas SQL sobre una base de datos educativa. Su objetivo es plantear retos progresivos, ejecutar las respuestas del usuario de forma segura y proporcionar una evaluación inmediata y comprensible.

### 2.1. Requisitos funcionales

El sistema debe cumplir los siguientes requisitos:

1. **Inicio de la sesión:** mostrar el título del simulador, las instrucciones básicas, el tiempo disponible y un botón para comenzar.
2. **Selección de retos:** cargar un banco de 30 retos organizados en cinco niveles de dificultad.
3. **Presentación del reto:** mostrar el enunciado, la puntuación, la categoría SQL y el editor donde el usuario introduce su consulta.
4. **Ejecución de consultas:** permitir ejecutar la respuesta y mostrar las columnas y filas obtenidas, limitando la visualización a 50 filas.
5. **Evaluación automática:** comparar el resultado de la consulta con la solución de referencia, teniendo en cuenta columnas, filas, tipos y valores.
6. **Retroalimentación:** informar si la respuesta es correcta o incorrecta y mostrar una explicación o pista cuando corresponda.
7. **Control del tiempo:** mantener un temporizador global de 300 segundos para toda la sesión.
8. **Puntuación:** acumular los puntos de los retos superados y mostrar el progreso durante la partida.
9. **Finalización:** bloquear nuevas respuestas cuando el tiempo se agote o cuando el usuario termine la sesión.
10. **Informe y exportación:** mostrar un resumen con aciertos, errores, puntuación y tiempo empleado, y permitir exportarlo en formatos JSON y CSV.

### 2.2. Banco de retos

El banco estará compuesto por 30 retos, distribuidos de forma progresiva:

| Nivel | Contenidos | Retos | Puntos por reto |
| :---: | :--- | :---: | :---: |
| **N1** | Selección y proyección con `SELECT` | 6 | 10 |
| **N2** | Filtrado con `WHERE`, condiciones y valores nulos | 6 | 20 |
| **N3** | Ordenación, cálculos y límites con `ORDER BY` y `LIMIT` | 6 | 30 |
| **N4** | Funciones de agregación: `COUNT`, `AVG`, `SUM`, `MIN` y `MAX` | 6 | 40 |
| **N5** | Agrupación avanzada con `GROUP BY` y `HAVING` | 6 | 50 |
| **Total** | **30 retos** | **30** | **900 puntos** |

Cada reto incluirá un identificador, un enunciado, el nivel de dificultad, la puntuación, las tablas implicadas, una consulta canónica de referencia y, cuando sea necesario, una explicación pedagógica. Las consultas se seleccionarán para practicar los contenidos del Tema 7 y se basarán en datos existentes en `davante_asir.db`.

### 2.3. Reglas de validación y seguridad

La validación se realizará en varias fases:

1. Comprobar que la entrada no está vacía.
2. Verificar que la consulta es de lectura y comienza con una sentencia `SELECT` válida.
3. Rechazar sentencias encadenadas separadas por punto y coma.
4. Ejecutar la consulta en una conexión SQLite abierta en modo solo lectura.
5. Aplicar un autorizador SQLite para bloquear operaciones de modificación de datos, cambios de estructura, pragmas y conexiones a otras bases de datos.
6. Comparar el resultado obtenido con el resultado esperado, sin exigir que el usuario utilice exactamente la misma sintaxis que la solución canónica.
7. Mostrar un error concreto cuando la consulta no pueda analizarse o ejecutarse, sin otorgar puntos.

La comparación tendrá en cuenta el número y el orden de las columnas, el número de filas y sus valores. Las diferencias de formato que no alteren el resultado lógico podrán considerarse equivalentes.

### 2.4. Flujo de navegación

El flujo principal de navegación será el siguiente:

```mermaid
flowchart TD
    A[Pantalla de inicio] --> B[Instrucciones]
    B --> C[Selección o carga del reto]
    C --> D[Editor SQL]
    D --> E{Ejecutar consulta}
    E -->|Correcta| F[Mostrar acierto y sumar puntos]
    E -->|Incorrecta| G[Mostrar error o pista]
    F --> H{¿Quedan retos y tiempo?}
    G --> H
    H -->|Sí| C
    H -->|No| I[Informe final]
    I --> J[Exportar resultados]
```

El usuario podrá avanzar al siguiente reto después de recibir la evaluación. El temporizador permanecerá visible durante toda la sesión y el final por tiempo llevará directamente al informe de resultados.

### 2.5. Interfaz de usuario

La interfaz se organizará en las siguientes zonas:

* **Cabecera:** nombre del simulador, logotipo del centro, temporizador y puntuación acumulada.
* **Panel de progreso:** niveles disponibles, reto actual y número de ejercicios completados.
* **Panel del enunciado:** descripción del reto, objetivos y tablas o campos relevantes.
* **Editor SQL:** área de texto con tipografía monoespaciada y botón para ejecutar la consulta.
* **Panel de resultados:** tabla con las filas devueltas, mensajes de validación, errores y pistas.
* **Pantalla final:** puntuación total, retos correctos e incorrectos, tiempo restante y botones de exportación.

La interfaz priorizará la legibilidad, el contraste y la identificación visual de los niveles mediante colores. Los mensajes de error serán claros y no mostrarán información interna innecesaria de la base de datos.

---

## 3. ANÁLISIS DE LA BASE DE DATOS (`davante_asir.db`)

La base de datos proporcionada representa el sistema integral de gestión docente, alumnado, instalaciones y recursos del centro formativo.

* **Tamaño del archivo:** 24,63 MB (25.227.264 bytes)
* **Motor:** SQLite 3 (embebido)
* **Volumen global:** 12 tablas relacionales y **425.100 filas** en total.

### 3.1. Diccionario de Datos

| Tabla | Registros | Clave Primaria | Claves Ajenas (FK) | Índices Secundarios |
| :--- | :---: | :--- | :--- | :--- |
| `alumnos` | 5.000 | `id_alumno` | Ninguna | Búsquedas por DNI/apellidos |
| `docentes` | 1.200 | `id_docente` | Ninguna | Filtrado por sede y situación laboral |
| `ofertas_formativas` | 1.500 | `id_oferta` | Ninguna | Ciclo, módulo y curso académico |
| `grupos` | 1.200 | `id_grupo` | `id_tutor` $\to$ `docentes`<br>`id_oferta` $\to$ `ofertas_formativas` | `codigo` (UNIQUE) |
| `matriculas` | 15.000 | `id_matricula` | `id_alumno` $\to$ `alumnos`<br>`id_grupo` $\to$ `grupos` | `idx_matriculas_alumno`<br>`idx_matriculas_grupo_estado` |
| `espacios` | 1.200 | `id_espacio` | Ninguna | `codigo` (UNIQUE) |
| `sesiones_clase` | 50.000 | `id_sesion` | `id_grupo` $\to$ `grupos`<br>`id_docente` $\to$ `docentes`<br>`id_espacio` $\to$ `espacios` | `idx_sesiones_grupo_fecha`<br>`idx_sesiones_docente_fecha` |
| `asistencias` | 200.000 | `id_asistencia`| `id_sesion` $\to$ `sesiones_clase`<br>`id_alumno` $\to$ `alumnos` | `idx_asistencias_sesion`<br>`idx_asistencias_alumno` |
| `actividades_evaluables` | 10.000 | `id_actividad` | `id_oferta` $\to$ `ofertas_formativas`<br>`id_docente` $\to$ `docentes` | `idx_actividades_oferta` |
| `calificaciones` | 100.000 | `id_calificacion`| `id_actividad` $\to$ `actividades_evaluables`<br>`id_alumno` $\to$ `alumnos` | `idx_calificaciones_actividad`<br>`idx_calificaciones_alumno` |
| `reservas_espacio` | 25.000 | `id_reserva` | `id_espacio` $\to$ `espacios`<br>`id_docente` $\to$ `docentes` | `idx_reservas_espacio` |
| `prestamos_equipos`| 15.000 | `id_prestamo` | `id_alumno` $\to$ `alumnos` | `idx_prestamos_alumno` |

### 3.2. Diagrama Entidad-Relación

```mermaid
erDiagram
    OFERTAS_FORMATIVAS ||--o{ GRUPOS : organiza
    OFERTAS_FORMATIVAS ||--o{ ACTIVIDADES_EVALUABLES : define
    DOCENTES ||--o{ GRUPOS : tutoriza
    DOCENTES ||--o{ SESIONES_CLASE : imparte
    DOCENTES ||--o{ ACTIVIDADES_EVALUABLES : califica
    DOCENTES ||--o{ RESERVAS_ESPACIO : solicita
    ESPACIOS ||--o{ SESIONES_CLASE : acoge
    ESPACIOS ||--o{ RESERVAS_ESPACIO : reserva
    GRUPOS ||--o{ MATRICULAS : contiene
    GRUPOS ||--o{ SESIONES_CLASE : programa
    ALUMNOS ||--o{ MATRICULAS : realiza
    ALUMNOS ||--o{ ASISTENCIAS : registra
    ALUMNOS ||--o{ CALIFICACIONES : obtiene
    ALUMNOS ||--o{ PRESTAMOS_EQUIPOS : solicita
    SESIONES_CLASE ||--o{ ASISTENCIAS : genera
    ACTIVIDADES_EVALUABLES ||--o{ CALIFICACIONES : evalua
```

---

## 4. ARQUITECTURA Y DECISIONES TÉCNICAS

### 4.1. ¿Por qué Rust y GPUI?
Para esta práctica se optó por una arquitectura de máxima eficiencia:
* **GPUI (DirectX 12):** Diseñado originalmente para el editor *Zed*, renderiza las interfaces calculando los fotogramas en la GPU sin navegadores embebidos (Chromium). Consume menos de **25 MB de RAM** frente a los 250 MB de entornos web/Electron.
* **Zero-Copy & Native C-ABI:** La interacción con `sqlite3` se produce en memoria de proceso nativa. Al evaluar consultas complejas sobre tablas de 200.000 filas, la deserialización toma **menos de 10 milisegundos**.
* **Identidad Visual Corporativa:** Integración del logotipo oficial del centro (`logotipo.png`) en el encabezado.
* **Binario único e independiente:** La aplicación compila a un archivo ejecutable `.exe` totalmente autónomo sin necesidad de intérpretes externos.

### 4.2. Medidas de Seguridad Implementadas
1. **URI de solo lectura estricta:** Conexión mediante `file:assets/davante_asir.db?mode=ro`.
2. **Autorizador SQLite nativo:** Mediante el hook `sqlite3_set_authorizer` en Rust, se intercepta cada paso de la preparación de la sentencia:
   * Permitido: `Action::Select`, `Action::Read`, `Action::Function`.
   * Bloqueado: `Action::Insert`, `Action::Update`, `Action::Delete`, `Action::DropTable`, `Action::AlterTable`, `Action::Pragma`, `Action::Attach`.
3. **Impedimento de sentencias encadenadas:** Se comprueba y rechaza cualquier entrada que contenga múltiples comandos separados por punto y coma (`;`).
4. **Límite de filas en pantalla:** Para evitar saturar el renderizado en consultas sin límite (ej. `SELECT * FROM asistencias`), la interfaz pagina y visualiza las primeras 50 filas, informando del total devuelto.

---

## 5. SISTEMA DE DIFICULTAD Y PUNTUACIÓN (900 PUNTOS)

| Nivel | Categoría Temática | Puntos / Reto | Total Nivel | Color Distintivo |
| :---: | :--- | :---: | :---: | :--- |
| **N1** | Selección y Proyección | 10 pts | 60 pts | Verde Esmeralda (`#10b981`) |
| **N2** | Filtrado con WHERE | 20 pts | 120 pts | Cian / Azul (`#06b6d4`) |
| **N3** | Ordenación y Límites | 30 pts | 180 pts | Violeta (`#8b5cf6`) |
| **N4** | Resumen y Agregación | 40 pts | 240 pts | Ámbar / Naranja (`#f59e0b`) |
| **N5** | Agrupación Avanzada (HAVING) | 50 pts | 300 pts | Rojo Coral (`#ef4444`) |
| **TOTAL** | **30 Retos Obligatorios** | - | **900 pts** | - |

---

## 6. TEMPORIZADOR GLOBAL Y EXPORTACIÓN FUNCIONAL

* **Temporizador Global Único (5 Minutos):** En lugar de otorgar 5 minutos por ejercicio, el participante dispone de **300 segundos globales** para resolver el máximo número de ejercicios posible, simulando un reto contrarreloj auténtico. Al agotarse el tiempo, la aplicación bloquea nuevas ejecuciones y abre automáticamente el informe de resultados.
* **Exportación en un Clic:** El botón de exportación genera de inmediato los archivos `exports/resultados_sprint_sql.json` y `exports/resultados_sprint_sql.csv`, presentando un diálogo con las rutas absolutas y un botón para abrir la carpeta directamente en el Explorador de archivos de Windows.
