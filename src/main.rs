#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use std::time::Duration;

mod app_state;
mod db;
mod export;
mod models;
mod validator;

use app_state::SimuladorState;
use export::Exporter;

const EMBEDDED_LOGO: &[u8] = include_bytes!("../logotipo.png");

struct SimuladorApp {
    state: SimuladorState,
    focus_handle: FocusHandle,
    logo_image: Arc<Image>,
}

impl SimuladorApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let logo_image = Arc::new(Image::from_bytes(ImageFormat::Png, EMBEDDED_LOGO.to_vec()));

        // Lanzar temporizador global en segundo plano (1 tick por segundo)
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let res = this.update(cx, |this, cx| {
                    this.state.tick_segundo();
                    cx.notify();
                });
                if res.is_err() {
                    break;
                }
            }
        })
        .detach();

        Self {
            state: SimuladorState::new(),
            focus_handle,
            logo_image,
        }
    }

    fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = &event.keystroke.key;
        let mods = &event.keystroke.modifiers;

        // Pantalla de inicio: permitir editar el nombre del participante si está activo
        if !self.state.sesion_iniciada {
            if self.state.editando_nombre {
                if key == "backspace" {
                    self.state.participante_nombre.pop();
                    cx.notify();
                } else if key == "enter" {
                    if self.state.participante_nombre.trim().is_empty() {
                        self.state.participante_nombre = "Alumno demo".to_string();
                    }
                    self.state.editando_nombre = false;
                    cx.notify();
                } else if key == "space" {
                    self.state.participante_nombre.push(' ');
                    cx.notify();
                } else if let Some(ref ch) = event.keystroke.key_char {
                    if !(mods.control && !mods.alt) && !mods.platform {
                        self.state.participante_nombre.push_str(ch);
                        cx.notify();
                    }
                }
            }
            return;
        }

        // Si la sesión ya finalizó (por tiempo o usuario), bloquear edición/envío SQL
        if self.state.sesion_finalizada {
            return;
        }

        // Atajos con Control (sin AltGr)
        if mods.control && !mods.alt && !mods.platform {
            if key == "enter" {
                self.state.ejecutar_y_comprobar();
                cx.notify();
                return;
            } else if key.eq_ignore_ascii_case("v") {
                if let Some(item) = cx.read_from_clipboard() {
                    if let Some(text) = item.text() {
                        self.state.sql_buffer.push_str(&text);
                        cx.notify();
                    }
                }
                return;
            } else if key.eq_ignore_ascii_case("c") {
                cx.write_to_clipboard(ClipboardItem::new_string(self.state.sql_buffer.clone()));
                return;
            } else if key == "backspace" {
                // Borrar última palabra o espacio
                while self.state.sql_buffer.ends_with(char::is_whitespace) {
                    self.state.sql_buffer.pop();
                }
                while self.state.sql_buffer.chars().last().is_some_and(|c| !c.is_whitespace()) {
                    self.state.sql_buffer.pop();
                }
                cx.notify();
                return;
            }
        }

        if key == "backspace" {
            self.state.sql_buffer.pop();
            cx.notify();
        } else if key == "enter" {
            self.state.sql_buffer.push('\n');
            cx.notify();
        } else if key == "space" {
            self.state.sql_buffer.push(' ');
            cx.notify();
        } else if key == "tab" {
            self.state.sql_buffer.push_str("    ");
            cx.notify();
        } else if let Some(ref ch) = event.keystroke.key_char {
            // key_char resuelve mayúsculas, acentos, símbolos (*, (, ), =, <, >) y AltGr (|) en teclado ES
            if !(mods.control && !mods.alt) && !mods.platform && !ch.is_empty() {
                self.state.sql_buffer.push_str(ch);
                cx.notify();
            }
        } else if !mods.control && !mods.alt && !mods.platform && key.len() == 1 {
            self.state.sql_buffer.push_str(key);
            cx.notify();
        }
    }

    /// Tabla reutilizable con desglose de los 5 niveles por categoría (sin puntos)
    fn render_level_breakdown_table(&self) -> Div {
        let niveles: [(u8, &str, u32); 5] = [
            (1, "Selección y Proyección", 0x10b981),
            (2, "Filtrado con WHERE", 0x06b6d4),
            (3, "Ordenación y Límites", 0x8b5cf6),
            (4, "Resumen y Agregación", 0xf59e0b),
            (5, "Agrupación Avanzada (HAVING)", 0xef4444),
        ];

        div()
            .flex()
            .flex_col()
            .gap_1()
            .rounded_lg()
            .overflow_hidden()
            .border_1()
            .border_color(rgb(0x1e293b))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .p_2()
                    .bg(rgb(0x1e293b))
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x94a3b8))
                    .child(div().w(px(80.0)).child("Nivel"))
                    .child(div().flex_1().child("Categoría"))
                    .child(div().w(px(100.0)).child("Superados"))
                    .child(div().w(px(110.0)).child("Estado")),
            )
            .children(niveles.into_iter().enumerate().map(|(i, (lvl, nombre, color))| {
                let total_lvl = self.state.retos.iter().filter(|r| r.nivel == lvl).count();
                let ok_lvl = self
                    .state
                    .retos
                    .iter()
                    .filter(|r| r.nivel == lvl && self.state.retos_superados.contains(&r.id))
                    .count();
                let row_bg = if i % 2 == 0 { rgb(0x131d31) } else { rgb(0x0f172a) };

                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .p_2()
                    .bg(row_bg)
                    .text_xs()
                    .text_color(rgb(color))
                    .child(div().w(px(80.0)).font_weight(FontWeight::BOLD).child(format!("Nivel {}", lvl)))
                    .child(div().flex_1().text_color(rgb(0xe2e8f0)).child(nombre))
                    .child(div().w(px(100.0)).font_weight(FontWeight::BOLD).child(format!("{}/{}", ok_lvl, total_lvl)))
                    .child(div().w(px(110.0)).child(if total_lvl > 0 && ok_lvl == total_lvl { "Completado" } else { "En curso" }))
            }))
    }

    /// Modal de confirmación de exportación funcional (reutilizado en sesión y pantalla final)
    fn render_export_modal(&self, cx: &mut Context<Self>) -> Div {
        let fallback_dir = Exporter::get_export_dir();
        let json_path = self
            .state
            .ruta_export_json
            .clone()
            .unwrap_or_else(|| fallback_dir.join("resultados.json").to_string_lossy().to_string());
        let csv_path = self
            .state
            .ruta_export_csv
            .clone()
            .unwrap_or_else(|| fallback_dir.join("resultados.csv").to_string_lossy().to_string());
        let aciertos = self.state.retos_superados.len();
        let total_retos = self.state.retos.len();
        let errores = self.state.errores_cometidos();

        div()
            .absolute()
            .size_full()
            .bg(rgba(0x000000d0))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(620.0))
                    .p_6()
                    .rounded_xl()
                    .bg(rgb(0x0f172a))
                    .border_1()
                    .border_color(rgb(0x059669))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_2()
                                    .child(div().text_lg().child("✅"))
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0x10b981))
                                            .child("¡Resultados Exportados con Éxito!"),
                                    ),
                            )
                            .child(
                                div()
                                    .id("btn_cerrar_modal_export")
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(rgb(0x94a3b8))
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.state.mostrar_modal_exportacion = false;
                                        cx.notify();
                                    }))
                                    .child("✕"),
                            ),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(0x131d31))
                            .border_1()
                            .border_color(rgb(0x1e293b))
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0x38bdf8))
                                    .child(format!(
                                        "Sesión: #{} · Participante: {}",
                                        self.state.sesion_id, self.state.participante_nombre
                                    )),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0x10b981))
                                    .child(format!(
                                        "• Resultado: {}/{} acertados (errores cometidos: {})",
                                        aciertos, total_retos, errores
                                    )),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xcbd5e1))
                                    .child(format!(
                                        "• Tiempo empleado: {:02}:{:02} (restante: {:02}:{:02})",
                                        self.state.tiempo_total_segundos / 60,
                                        self.state.tiempo_total_segundos % 60,
                                        self.state.tiempo_restante_global / 60,
                                        self.state.tiempo_restante_global % 60
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0x94a3b8))
                                    .child("ARCHIVOS GENERADOS:"),
                            )
                            .child(
                                div()
                                    .p_2()
                                    .rounded_sm()
                                    .bg(rgb(0x1e293b))
                                    .text_xs()
                                    .text_color(rgb(0x67e8f9))
                                    .child(format!("📄 JSON: {}", json_path)),
                            )
                            .child(
                                div()
                                    .p_2()
                                    .rounded_sm()
                                    .bg(rgb(0x1e293b))
                                    .text_xs()
                                    .text_color(rgb(0x86efac))
                                    .child(format!("📊 CSV:  {}", csv_path)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .gap_3()
                            .child(
                                div()
                                    .id("btn_abrir_carpeta_export")
                                    .px_4()
                                    .py_2()
                                    .rounded_md()
                                    .bg(rgb(0x2563eb))
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x1d4ed8)))
                                    .on_click(cx.listener(|this, _event, _window, _cx| {
                                        this.state.abrir_carpeta_exportacion();
                                    }))
                                    .child("📁 Abrir Carpeta con los Archivos"),
                            )
                            .child(
                                div()
                                    .id("btn_cerrar_export_modal")
                                    .px_4()
                                    .py_2()
                                    .rounded_md()
                                    .bg(rgb(0x334155))
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x475569)))
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.state.mostrar_modal_exportacion = false;
                                        cx.notify();
                                    }))
                                    .child("Cerrar"),
                            ),
                    ),
            )
    }

    /// 1. Pantalla de Inicio: título, instrucciones básicas, tiempo disponible y botón Comenzar
    fn render_start_screen(&self, cx: &mut Context<Self>) -> Div {
        let total_retos = self.state.retos.len();

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .p_6()
            .child(
                div()
                    .w(px(760.0))
                    .p_6()
                    .rounded_xl()
                    .bg(rgb(0x0f172a))
                    .border_1()
                    .border_color(rgb(0x1e293b))
                    .flex()
                    .flex_col()
                    .gap_5()
                    // Cabecera con logotipo y título del simulador
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .pb_4()
                            .border_b_1()
                            .border_color(rgb(0x1e293b))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_4()
                                    .child(
                                        img(self.logo_image.clone())
                                            .w(px(64.0))
                                            .h(px(53.0))
                                            .rounded_sm(),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(0x38bdf8))
                                                    .child("INSTITUTO DAVANTE · ASIR · TEMA 7 BDD"),
                                            )
                                            .child(
                                                div()
                                                    .text_xl()
                                                    .font_weight(FontWeight::BOLD)
                                                    .child("SQL Sprint Davante — Simulador Interactivo"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x94a3b8))
                                                    .child("Programa realizado por Hector, Aya y Nerea"),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .bg(rgb(0x1e293b))
                                    .border_1()
                                    .border_color(rgb(0x38bdf8))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .child(div().text_xs().text_color(rgb(0x94a3b8)).child("TIEMPO DISPONIBLE"))
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0x38bdf8))
                                            .child("⏱ 05:00 (300 s)"),
                                    ),
                            ),
                    )
                    // Instrucciones básicas
                    .child(
                        div()
                            .p_4()
                            .rounded_lg()
                            .bg(rgb(0x131d31))
                            .border_1()
                            .border_color(rgb(0x1e293b))
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xfbbf24))
                                    .child("INSTRUCCIONES BÁSICAS DE LA PRÁCTICA"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xe2e8f0))
                                    .child(format!(
                                        "1. El simulador incluye {} retos SQL progresivos agrupados en 5 categorías (evaluación directa por aciertos, ej. 28/30).",
                                        total_retos
                                    )),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xe2e8f0))
                                    .child("2. Escribe tu consulta SELECT directamente en el editor o mediante la paleta rápida y el esquema lateral."),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xe2e8f0))
                                    .child("3. Seguridad solo lectura: solo se admiten consultas SELECT únicas sobre la base de datos educativa (máx. 50 filas visibles)."),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xe2e8f0))
                                    .child("4. Pulsa 'Ejecutar y comprobar' (o Ctrl+Enter) para validar columnas, filas, tipos y valores, y ver pistas si falla."),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xe2e8f0))
                                    .child("5. Dispones de 300 segundos globales. Los resultados JSON/CSV se guardarán en Documentos\\BD_Hector_Nerea_Deborah\\."),
                            ),
                    )
                    // Tabla resumen de niveles
                    .child(self.render_level_breakdown_table())
                    // Identificación del alumno y botón Comenzar
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .pt_2()
                            .child(
                                div()
                                    .id("btn_edit_participante")
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .bg(rgb(0x131d31))
                                    .border_1()
                                    .border_color(if self.state.editando_nombre {
                                        rgb(0x38bdf8)
                                    } else {
                                        rgb(0x334155)
                                    })
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.state.editando_nombre = true;
                                        cx.notify();
                                    }))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap_2()
                                            .text_xs()
                                            .child(div().text_color(rgb(0x94a3b8)).child("👤 Alumno/a:"))
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(0xf8fafc))
                                                    .child(if self.state.editando_nombre {
                                                        format!("{}█", self.state.participante_nombre)
                                                    } else {
                                                        format!("{} (clic para editar)", self.state.participante_nombre)
                                                    }),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .id("btn_comenzar_sesion")
                                    .px_6()
                                    .py_3()
                                    .rounded_lg()
                                    .bg(rgb(0x2563eb))
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x1d4ed8)))
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        if this.state.participante_nombre.trim().is_empty() {
                                            this.state.participante_nombre = "Alumno demo".to_string();
                                        }
                                        this.state.iniciar_sesion();
                                        cx.notify();
                                    }))
                                    .child("▶ Comenzar Sesión (Iniciar 05:00)"),
                            ),
                    ),
            )
    }

    /// 3. Pantalla Final: calificación X/30, retos correctos/incorrectos, tiempo empleado/restante y exportación (sin puntos)
    fn render_final_screen(&self, cx: &mut Context<Self>) -> Div {
        let aciertos = self.state.retos_superados.len();
        let total_retos = self.state.retos.len();
        let incorrectos = self.state.retos_no_superados();
        let errores = self.state.errores_cometidos();
        let tiempo_emp_m = self.state.tiempo_total_segundos / 60;
        let tiempo_emp_s = self.state.tiempo_total_segundos % 60;
        let tiempo_res_m = self.state.tiempo_restante_global / 60;
        let tiempo_res_s = self.state.tiempo_restante_global % 60;
        let por_tiempo = self.state.tiempo_restante_global == 0;

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .p_6()
            .child(
                div()
                    .w(px(780.0))
                    .p_6()
                    .rounded_xl()
                    .bg(rgb(0x0f172a))
                    .border_1()
                    .border_color(if por_tiempo { rgb(0xf59e0b) } else { rgb(0x10b981) })
                    .flex()
                    .flex_col()
                    .gap_5()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .pb_4()
                            .border_b_1()
                            .border_color(rgb(0x1e293b))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_3()
                                    .child(img(self.logo_image.clone()).w(px(52.0)).h(px(43.0)).rounded_sm())
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(if por_tiempo { rgb(0xf59e0b) } else { rgb(0x10b981) })
                                                    .child(if por_tiempo {
                                                        "⏱ TIEMPO GLOBAL AGOTADO · RESPUESTAS BLOQUEADAS"
                                                    } else {
                                                        "🏁 INFORME FINAL DE LA SESIÓN · RESPUESTAS BLOQUEADAS"
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .text_lg()
                                                    .font_weight(FontWeight::BOLD)
                                                    .child(format!(
                                                        "Resultados de {} (Sesión #{})",
                                                        self.state.participante_nombre, self.state.sesion_id
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x94a3b8))
                                                    .child("Programa realizado por Hector, Aya y Nerea"),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .px_4()
                                    .py_2()
                                    .rounded_lg()
                                    .bg(rgb(0x131d31))
                                    .border_1()
                                    .border_color(rgb(0x10b981))
                                    .text_lg()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0x10b981))
                                    .child(format!("✓ {}/{}", aciertos, total_retos)),
                            ),
                    )
                    // Tarjetas resumen exigidas: correctos, incorrectos, errores, tiempo empleado y restante
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .p_3()
                                    .rounded_lg()
                                    .bg(rgb(0x131d31))
                                    .border_1()
                                    .border_color(rgb(0x10b981))
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(div().text_xs().text_color(rgb(0x94a3b8)).child("Retos correctos"))
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0x10b981))
                                            .child(format!("{}/{}", aciertos, total_retos)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .p_3()
                                    .rounded_lg()
                                    .bg(rgb(0x131d31))
                                    .border_1()
                                    .border_color(rgb(0xef4444))
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(div().text_xs().text_color(rgb(0x94a3b8)).child("No superados / Fallos"))
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0xef4444))
                                            .child(format!("{} ({} errores)", incorrectos, errores)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .p_3()
                                    .rounded_lg()
                                    .bg(rgb(0x131d31))
                                    .border_1()
                                    .border_color(rgb(0x38bdf8))
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(div().text_xs().text_color(rgb(0x94a3b8)).child("Tiempo empleado"))
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0x38bdf8))
                                            .child(format!("{:02}:{:02}", tiempo_emp_m, tiempo_emp_s)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .p_3()
                                    .rounded_lg()
                                    .bg(rgb(0x131d31))
                                    .border_1()
                                    .border_color(rgb(0xa78bfa))
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(div().text_xs().text_color(rgb(0x94a3b8)).child("Tiempo restante"))
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0xa78bfa))
                                            .child(format!("{:02}:{:02}", tiempo_res_m, tiempo_res_s)),
                                    ),
                            ),
                    )
                    // Desglose de categorías (sin puntos)
                    .child(self.render_level_breakdown_table())
                    // Botones finales de exportación y reinicio
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .pt_2()
                            .child(
                                div()
                                    .id("btn_reiniciar_final")
                                    .px_4()
                                    .py_2()
                                    .rounded_md()
                                    .bg(rgb(0x334155))
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x475569)))
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.state.reiniciar_partida();
                                        cx.notify();
                                    }))
                                    .child("↺ Iniciar Nueva Sesión"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_3()
                                    .child(
                                        div()
                                            .id("btn_export_final")
                                            .px_5()
                                            .py_2()
                                            .rounded_md()
                                            .bg(rgb(0x059669))
                                            .text_sm()
                                            .font_weight(FontWeight::BOLD)
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgb(0x10b981)))
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.state.exportar_resultados();
                                                cx.notify();
                                            }))
                                            .child("💾 Exportar Informe (JSON y CSV)"),
                                    ),
                            ),
                    ),
            )
    }
}

impl Render for SimuladorApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x0b0f19)) // Fondo general oscuro moderno
            .text_color(rgb(0xf1f5f9))
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                this.handle_key(event, cx);
            }));

        // 1. Pantalla de inicio e instrucciones antes de comenzar la sesión
        if !self.state.sesion_iniciada {
            return root.child(self.render_start_screen(cx));
        }

        // 2. Pantalla final (informe completo y bloqueo de nuevas respuestas)
        if self.state.sesion_finalizada {
            return root
                .child(self.render_final_screen(cx))
                .when(self.state.mostrar_modal_exportacion, |this| {
                    this.child(self.render_export_modal(cx))
                });
        }

        let reto_actual = self.state.reto_actual().cloned();
        let total_retos = self.state.retos.len();
        let actual_idx = self.state.reto_actual_idx;
        let aciertos = self.state.retos_superados.len();
        let intentos = self.state.historial_intentos.len();
        let errores = self.state.errores_cometidos();

        // Temporizador GLOBAL de 5 minutos
        let minutos = self.state.tiempo_restante_global / 60;
        let segundos = self.state.tiempo_restante_global % 60;
        let timer_color = if self.state.tiempo_restante_global == 0 {
            rgb(0xef4444) // Rojo fin
        } else if self.state.tiempo_restante_global <= 30 {
            rgb(0xef4444) // Rojo alerta
        } else if self.state.tiempo_restante_global <= 60 {
            rgb(0xf59e0b) // Amarillo aviso
        } else {
            rgb(0x38bdf8) // Azul normal
        };

        root
            // =========================================================================
            // 1. CABECERA PRINCIPAL CON LOGO, TIEMPO GLOBAL Y ACCIONES (SIN PUNTOS)
            // =========================================================================
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .h(px(64.0))
                    .px_6()
                    .bg(rgb(0x0f172a))
                    .border_b_1()
                    .border_color(rgb(0x1e293b))
                    // Lado izquierdo: Logotipo + Nombre del centro, app y autores
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(
                                img(self.logo_image.clone())
                                    .w(px(46.0))
                                    .h(px(38.0))
                                    .rounded_sm(),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0x38bdf8))
                                            .child("INSTITUTO DAVANTE · ASIR"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::BOLD)
                                            .child("SQL Sprint (GPU Accelerated)"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0x94a3b8))
                                            .child("Programa realizado por Hector, Aya y Nerea"),
                                    ),
                            ),
                    )
                    // Lado derecho: Temporizador Global, Participante y Acciones (sin puntos)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            // Cronómetro Único Global (5 minutos)
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x1e293b))
                                    .border_1()
                                    .border_color(timer_color)
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(timer_color)
                                    .child(format!("⏱ Global: {:02}:{:02}", minutos, segundos)),
                            )
                            // Identificación del alumno (sin equipos)
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x1e293b))
                                    .text_xs()
                                    .child(format!("👤 {}", self.state.participante_nombre)),
                            )
                            // Botón Clasificación
                            .child(
                                div()
                                    .id("btn_clasificacion")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x0369a1))
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x0284c7)))
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.state.mostrar_modal_ranking = !this.state.mostrar_modal_ranking;
                                        cx.notify();
                                    }))
                                    .child("🏆 Clasificación"),
                            )
                            // Botón Exportar Funcional
                            .child(
                                div()
                                    .id("btn_exportar")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x059669))
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x10b981)))
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.state.exportar_resultados();
                                        cx.notify();
                                    }))
                                    .child("💾 Exportar CSV/JSON"),
                            )
                            // Botón Finalizar sesión manualmente
                            .child(
                                div()
                                    .id("btn_finalizar_sesion")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0xb91c1c))
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0xdc2626)))
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.state.finalizar_sesion(false);
                                        cx.notify();
                                    }))
                                    .child("🏁 Finalizar"),
                            ),
                    ),
            )
            // =========================================================================
            // 2. CONTENIDO PRINCIPAL EN TRES COLUMNAS
            // =========================================================================
            .child(
                div()
                    .flex()
                    .flex_row()
                    .size_full()
                    .overflow_hidden()
                    // -----------------------------------------------------------------
                    // COLUMNA IZQUIERDA: BANCO DE RETOS POR CATEGORÍAS Y DIFICULTAD
                    // -----------------------------------------------------------------
                    .child(
                        div()
                            .w(px(290.0))
                            .h_full()
                            .flex()
                            .flex_col()
                            .bg(rgb(0x0f172a))
                            .border_r_1()
                            .border_color(rgb(0x1e293b))
                            // Cabecera del banco de retos (sin puntos en lateral)
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .p_3()
                                    .border_b_1()
                                    .border_color(rgb(0x1e293b))
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0x94a3b8))
                                            .child("BANCO DE RETOS"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(if aciertos == total_retos {
                                                rgb(0x10b981)
                                            } else {
                                                rgb(0x38bdf8)
                                            })
                                            .child(format!("{}/{} superados", aciertos, total_retos)),
                                    ),
                            )
                            // Lista de retos organizada por categorías extensibles / contraíbles
                            .child(
                                div()
                                    .id("retos_list")
                                    .flex()
                                    .flex_col()
                                    .overflow_y_scroll()
                                    .p_2()
                                    .gap_2()
                                    .children((1..=5u8).map(|lvl| {
                                        let cat_retos: Vec<(usize, &models::Reto)> = self.state.retos
                                            .iter()
                                            .enumerate()
                                            .filter(|(_, r)| r.nivel == lvl)
                                            .collect();

                                        let cat_nombre = match lvl {
                                            1 => "1. Selección y Proyección",
                                            2 => "2. Filtrado con WHERE",
                                            3 => "3. Ordenación y Límites",
                                            4 => "4. Resumen y Agregación",
                                            5 => "5. Agrupación (HAVING)",
                                            _ => "Categoría",
                                        };

                                        let cat_color = match lvl {
                                            1 => 0x10b981,
                                            2 => 0x06b6d4,
                                            3 => 0x8b5cf6,
                                            4 => 0xf59e0b,
                                            5 => 0xef4444,
                                            _ => 0x38bdf8,
                                        };

                                        let cat_resueltos = cat_retos
                                            .iter()
                                            .filter(|(_, r)| self.state.retos_superados.contains(&r.id))
                                            .count();

                                        let is_expanded = *self.state.categorias_expandidas.get(&lvl).unwrap_or(&true);
                                        let chevron = if is_expanded { "▼" } else { "▶" };
                                        let cat_completed = cat_resueltos == cat_retos.len() && !cat_retos.is_empty();

                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                // Cabecera clickeable de categoría (extender / contraer)
                                                div()
                                                    .id(ElementId::NamedInteger("cat_header".into(), lvl as u64))
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .justify_between()
                                                    .px_2()
                                                    .py_1()
                                                    .rounded_md()
                                                    .bg(rgb(0x131d31))
                                                    .border_1()
                                                    .border_color(rgb(0x1e293b))
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(rgb(0x1e293b)))
                                                    .on_click(cx.listener(move |this, _event, _window, cx| {
                                                        this.state.toggle_categoria(lvl);
                                                        cx.notify();
                                                    }))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_row()
                                                            .items_center()
                                                            .gap_2()
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(rgb(0x94a3b8))
                                                                    .child(chevron),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .font_weight(FontWeight::BOLD)
                                                                    .text_color(rgb(cat_color))
                                                                    .child(cat_nombre),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .px_2()
                                                            .py_0p5()
                                                            .rounded_sm()
                                                            .bg(if cat_completed { rgb(0x064e3b) } else { rgb(0x0f172a) })
                                                            .text_xs()
                                                            .font_weight(FontWeight::BOLD)
                                                            .text_color(if cat_completed { rgb(0x34d399) } else { rgb(0x94a3b8) })
                                                            .child(if cat_completed {
                                                                format!("✓ {}/{}", cat_resueltos, cat_retos.len())
                                                            } else {
                                                                format!("{}/{}", cat_resueltos, cat_retos.len())
                                                            }),
                                                    ),
                                            )
                                            .when(is_expanded, |this| {
                                                this.child(
                                                    div()
                                                        .flex()
                                                        .flex_col()
                                                        .gap_1()
                                                        .mt_1()
                                                        .children(cat_retos.into_iter().map(|(idx, r)| {
                                                            let is_selected = idx == actual_idx;
                                                            let is_solved = self.state.retos_superados.contains(&r.id);

                                                            let bg_color = if is_selected {
                                                                rgb(0x1e293b)
                                                            } else {
                                                                rgb(0x0f172a)
                                                            };

                                                            let icon = if is_solved {
                                                                "✓ "
                                                            } else if is_selected {
                                                                "▶ "
                                                            } else {
                                                                "• "
                                                            };

                                                            let icon_color = if is_solved {
                                                                rgb(0x10b981)
                                                            } else if is_selected {
                                                                rgb(0x38bdf8)
                                                            } else {
                                                                rgb(0x64748b)
                                                            };

                                                            let text_color = if is_selected {
                                                                rgb(0xf8fafc)
                                                            } else if is_solved {
                                                                rgb(0xcbd5e1)
                                                            } else {
                                                                rgb(0x94a3b8)
                                                            };

                                                            div()
                                                                .id(ElementId::NamedInteger("reto_item".into(), idx as u64))
                                                                .flex()
                                                                .flex_row()
                                                                .items_center()
                                                                .justify_between()
                                                                .p_2()
                                                                .rounded_md()
                                                                .bg(bg_color)
                                                                .border_l_2()
                                                                .border_color(if is_selected {
                                                                    rgb(cat_color)
                                                                } else {
                                                                    rgb(0x0f172a)
                                                                })
                                                                .cursor_pointer()
                                                                .hover(|s| s.bg(rgb(0x273549)))
                                                                .on_click(cx.listener(move |this, _event, _window, cx| {
                                                                    this.state.seleccionar_reto(idx);
                                                                    cx.notify();
                                                                }))
                                                                .child(
                                                                    div()
                                                                        .flex()
                                                                        .flex_row()
                                                                        .items_center()
                                                                        .gap_2()
                                                                        .child(
                                                                            div()
                                                                                .text_xs()
                                                                                .font_weight(FontWeight::BOLD)
                                                                                .text_color(icon_color)
                                                                                .child(icon),
                                                                        )
                                                                        .child(
                                                                            div()
                                                                                .text_xs()
                                                                                .font_weight(if is_selected {
                                                                                    FontWeight::BOLD
                                                                                } else {
                                                                                    FontWeight::NORMAL
                                                                                })
                                                                                .text_color(text_color)
                                                                                .child(format!("{}: {}", r.id, r.titulo)),
                                                                        ),
                                                                )
                                                        })),
                                                )
                                            })
                                    })),
                            ),
                    )
                    // -----------------------------------------------------------------
                    // COLUMNA CENTRAL: ENUNCIADO, PALETA, EDITOR Y RESULTADOS
                    // -----------------------------------------------------------------
                    .child(
                        div()
                            .id("center_scroll_view")
                            .flex()
                            .flex_1()
                            .flex_col()
                            .h_full()
                            .overflow_y_scroll()
                            .p_4()
                            .gap_4()
                            // --- FICHA DEL RETO CON CATEGORÍA Y DIFICULTAD DESTACADA ---
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .p_4()
                                    .rounded_lg()
                                    .bg(rgb(0x131d31))
                                    .border_1()
                                    .border_color(rgb(0x1e293b))
                                    .gap_3()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_lg()
                                                    .font_weight(FontWeight::BOLD)
                                                    .child(if let Some(ref r) = reto_actual {
                                                        format!("{}: {}", r.id, r.titulo)
                                                    } else {
                                                        "Sin reto seleccionado".to_string()
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap_3()
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(rgb(0x94a3b8))
                                                            .child(format!("Reto {} de {}", actual_idx + 1, total_retos)),
                                                    )
                                                    // Tiempo Global Restante
                                                    .child(
                                                        div()
                                                            .px_3()
                                                            .py_1()
                                                            .rounded_md()
                                                            .bg(rgb(0x1e293b))
                                                            .border_1()
                                                            .border_color(timer_color)
                                                            .text_sm()
                                                            .font_weight(FontWeight::BOLD)
                                                            .text_color(timer_color)
                                                            .child(format!("⏱ {:02}:{:02} global", minutos, segundos)),
                                                    ),
                                            ),
                                    )
                                    // Categoría y Enunciado
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap_2()
                                            .when_some(reto_actual.as_ref(), |this, r| {
                                                this.child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(rgb(r.color_hex()))
                                                        .child(format!("📁 Categoría: {}", r.categoria())),
                                                )
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(rgb(0xe2e8f0))
                                            .child(if let Some(ref r) = reto_actual {
                                                r.peticion.clone()
                                            } else {
                                                "".to_string()
                                            }),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap_4()
                                            .text_xs()
                                            .child(
                                                div()
                                                    .text_color(rgb(0x38bdf8))
                                                    .child(format!("Tabla: {}", reto_actual.as_ref().map(|r| r.tabla.as_str()).unwrap_or("-"))),
                                            )
                                            .child(
                                                div()
                                                    .text_color(rgb(0xa78bfa))
                                                    .child(format!("Elementos SQL: {}", reto_actual.as_ref().map(|r| r.elementos_sql.as_str()).unwrap_or("-"))),
                                            )
                                            .child(
                                                div()
                                                    .id("toggle_pistas")
                                                    .cursor_pointer()
                                                    .text_color(rgb(0xfbbf24))
                                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                                        this.state.mostrar_pistas = !this.state.mostrar_pistas;
                                                        cx.notify();
                                                    }))
                                                    .child(if self.state.mostrar_pistas { "▲ Ocultar pistas" } else { "💡 Ver pistas del reto" }),
                                            ),
                                    )
                                    // Pistas desplegables
                                    .when(self.state.mostrar_pistas, |this| {
                                        let pistas = reto_actual.as_ref().map(|r| r.pistas.clone()).unwrap_or_default();
                                        this.child(
                                            div()
                                                .p_3()
                                                .rounded_md()
                                                .bg(rgb(0x1e293b))
                                                .border_1()
                                                .border_color(rgb(0x334155))
                                                .flex()
                                                .flex_col()
                                                .gap_1()
                                                .children(pistas.into_iter().map(|p| {
                                                    div().text_xs().text_color(rgb(0xfef08a)).child(format!("• {}", p))
                                                })),
                                        )
                                    }),
                            )
                            // --- PALETA VISIBLE DE ELEMENTOS SQL ---
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(0x94a3b8)).child("PALETA RÁPIDA SQL (Clic para insertar)"))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .flex_wrap()
                                            .gap_1()
                                            .children(
                                                vec![
                                                    "SELECT", "FROM", "WHERE", "ORDER BY", "GROUP BY", "HAVING", "LIMIT",
                                                    "DISTINCT", "AS", "COUNT(*)", "AVG(", "SUM(", "MIN(", "MAX(",
                                                    "AND", "OR", "LIKE", "BETWEEN", "IS NULL", "DESC", "ASC", "||"
                                                ].into_iter().enumerate().map(|(t_idx, token)| {
                                                    div()
                                                        .id(ElementId::NamedInteger("sql_token".into(), t_idx as u64))
                                                        .px_2()
                                                        .py_1()
                                                        .rounded_sm()
                                                        .bg(rgb(0x1e293b))
                                                        .text_xs()
                                                        .text_color(rgb(0x38bdf8))
                                                        .cursor_pointer()
                                                        .hover(|s| s.bg(rgb(0x334155)))
                                                        .on_click(cx.listener(move |this, _event, _window, cx| {
                                                            this.state.sql_buffer.push_str(token);
                                                            this.state.sql_buffer.push(' ');
                                                            cx.notify();
                                                        }))
                                                        .child(token)
                                                })
                                            ),
                                    ),
                            )
                            // --- CONSTRUCTOR / EDITOR SQL ---
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .rounded_lg()
                                    .bg(rgb(0x0f172a))
                                    .border_1()
                                    .border_color(rgb(0x1e293b))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .justify_between()
                                            .px_3()
                                            .py_2()
                                            .bg(rgb(0x131d31))
                                            .border_b_1()
                                            .border_color(rgb(0x1e293b))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(0x94a3b8))
                                                    .child("CONSULTA SQL (Escribe directamente o pulsa en la paleta)"),
                                            )
                                            .child(
                                                div()
                                                    .id("btn_clear_sql")
                                                    .px_2()
                                                    .py_1()
                                                    .rounded_sm()
                                                    .bg(rgb(0x334155))
                                                    .text_xs()
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(rgb(0x475569)))
                                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                                        this.state.sql_buffer.clear();
                                                        cx.notify();
                                                    }))
                                                    .child("Limpiar"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .p_3()
                                            .min_h(px(100.0))
                                            .font_family("Consolas")
                                            .text_sm()
                                            .child(
                                                div()
                                                    .text_color(rgb(0x38bdf8))
                                                    .child(format!("{}█", self.state.sql_buffer)),
                                            ),
                                    ),
                            )
                            // --- BOTONES DE ACCIÓN ---
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .id("btn_ejecutar")
                                            .px_4()
                                            .py_2()
                                            .rounded_md()
                                            .bg(rgb(0x2563eb))
                                            .text_sm()
                                            .font_weight(FontWeight::BOLD)
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgb(0x1d4ed8)))
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.state.ejecutar_y_comprobar();
                                                cx.notify();
                                            }))
                                            .child("▶ Ejecutar y comprobar"),
                                    )
                                    .child(
                                        div()
                                            .id("btn_siguiente")
                                            .px_4()
                                            .py_2()
                                            .rounded_md()
                                            .bg(rgb(0x334155))
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgb(0x475569)))
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.state.siguiente_reto();
                                                cx.notify();
                                            }))
                                            .child(if actual_idx + 1 < total_retos {
                                                "Siguiente reto ⏭"
                                            } else {
                                                "🏁 Finalizar y ver informe"
                                            }),
                                    ),
                            )
                            // --- TABLA Y ESTADO DE RESULTADOS DE LA CONSULTA ---
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .rounded_lg()
                                    .bg(rgb(0x0f172a))
                                    .border_1()
                                    .border_color(match &self.state.status_feedback {
                                        Some((true, _)) => rgb(0x059669),
                                        Some((false, _)) => rgb(0xd97706),
                                        None => rgb(0x1e293b),
                                    })
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .justify_between()
                                            .px_3()
                                            .py_2()
                                            .bg(rgb(0x131d31))
                                            .border_b_1()
                                            .border_color(rgb(0x1e293b))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(0x94a3b8))
                                                    .child("RESULTADO DE LA CONSULTA"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x38bdf8))
                                                    .child(if let Some(ref q) = self.state.query_result {
                                                        if q.is_truncated {
                                                            format!(
                                                                "{:.2} ms | Mostrando 50 primeras filas (de {} totales)",
                                                                q.execution_time_ms, q.total_rows
                                                            )
                                                        } else {
                                                            format!("{:.2} ms | {} filas", q.execution_time_ms, q.total_rows)
                                                        }
                                                    } else {
                                                        "Esperando ejecución...".to_string()
                                                    }),
                                            ),
                                    )
                                    .when_some(self.state.status_feedback.clone(), |this, (es_exito, msg)| {
                                        let (bg_color, txt_col) = if es_exito {
                                            (rgb(0x064e3b), rgb(0x6ee7b7))
                                        } else {
                                            (rgb(0x451a03), rgb(0xfde68a))
                                        };
                                        this.child(
                                            div()
                                                .px_3()
                                                .py_2()
                                                .bg(bg_color)
                                                .border_b_1()
                                                .border_color(rgb(0x1e293b))
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(txt_col)
                                                .child(msg),
                                        )
                                    })
                                    .child(
                                        div()
                                            .id("result_table")
                                            .max_h(px(260.0))
                                            .overflow_scroll()
                                            .p_2()
                                            .child(if let Some(ref q) = self.state.query_result {
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    // Cabecera de columnas
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_row()
                                                            .gap_2()
                                                            .p_2()
                                                            .border_b_1()
                                                            .border_color(rgb(0x334155))
                                                            .children(q.columns.iter().map(|c| {
                                                                div()
                                                                    .w(px(140.0))
                                                                    .flex_shrink_0()
                                                                    .text_xs()
                                                                    .font_weight(FontWeight::BOLD)
                                                                    .text_color(rgb(0x93c5fd))
                                                                    .child(c.clone())
                                                            })),
                                                    )
                                                    // Filas de datos
                                                    .children(q.rows.iter().enumerate().map(|(idx, row)| {
                                                        let row_bg = if idx % 2 == 0 { rgb(0x0f172a) } else { rgb(0x131d31) };
                                                        div()
                                                            .flex()
                                                            .flex_row()
                                                            .gap_2()
                                                            .p_2()
                                                            .bg(row_bg)
                                                            .children(row.iter().map(|val| {
                                                                div()
                                                                    .w(px(140.0))
                                                                    .flex_shrink_0()
                                                                    .text_xs()
                                                                    .text_color(rgb(0xcbd5e1))
                                                                    .child(val.clone())
                                                            }))
                                                    }))
                                            } else {
                                                div()
                                                    .p_4()
                                                    .text_xs()
                                                    .text_color(rgb(0x64748b))
                                                    .child("Pulsa 'Ejecutar y comprobar' para ver el resultado aquí.")
                                            }),
                                    ),
                            ),
                    )
                    // -----------------------------------------------------------------
                    // COLUMNA DERECHA: ESQUEMA Y MÉTRICAS
                    // -----------------------------------------------------------------
                    .child(
                        div()
                            .w(px(280.0))
                            .h_full()
                            .flex()
                            .flex_col()
                            .bg(rgb(0x0f172a))
                            .border_l_1()
                            .border_color(rgb(0x1e293b))
                            // Tarjeta de métricas (solo aciertos/errores y tiempo global, sin puntos)
                            .child(
                                div()
                                    .p_3()
                                    .border_b_1()
                                    .border_color(rgb(0x1e293b))
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(0x94a3b8)).child("ESTADÍSTICAS SPRINT"))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .justify_between()
                                            .child(div().text_xs().text_color(rgb(0xcbd5e1)).child("Retos superados:"))
                                            .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(0x10b981)).child(format!("{}/{}", aciertos, total_retos))),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .justify_between()
                                            .child(div().text_xs().text_color(rgb(0xcbd5e1)).child("Intentos / Errores:"))
                                            .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(0x38bdf8)).child(format!("{} / {}", intentos, errores))),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .justify_between()
                                            .child(div().text_xs().text_color(rgb(0xcbd5e1)).child("Tiempo restante (global):"))
                                            .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(timer_color).child(format!("{:02}:{:02}", minutos, segundos))),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .justify_between()
                                            .child(div().text_xs().text_color(rgb(0xcbd5e1)).child("Tiempo total empleado:"))
                                            .child(div().text_xs().font_weight(FontWeight::BOLD).child(format!("{:02}:{:02}", self.state.tiempo_total_segundos / 60, self.state.tiempo_total_segundos % 60))),
                                    )
                                    .child(
                                        div()
                                            .id("btn_reiniciar_partida")
                                            .p_1()
                                            .rounded_sm()
                                            .bg(rgb(0x334155))
                                            .text_xs()
                                            .text_color(rgb(0xf87171))
                                            .text_center()
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgb(0x475569)))
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.state.reiniciar_partida();
                                                cx.notify();
                                            }))
                                            .child("↺ Reiniciar Partida (5 min nuevos)"),
                                    ),
                            )
                            // Árbol del esquema de la base de datos
                            .child(
                                div()
                                    .p_3()
                                    .border_b_1()
                                    .border_color(rgb(0x1e293b))
                                    .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(0x94a3b8)).child("ESQUEMA (Solo lectura - mode=ro)")),
                            )
                            .child(
                                div()
                                    .id("schema_tree")
                                    .flex()
                                    .flex_col()
                                    .overflow_y_scroll()
                                    .p_2()
                                    .gap_1()
                                    .children(self.state.schema.iter().enumerate().map(|(s_idx, tbl)| {
                                        let tbl_name = tbl.name.clone();
                                        let is_expanded = *self.state.tablas_expandidas.get(&tbl_name).unwrap_or(&false);
                                        let cols = tbl.columns.clone();

                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .id(ElementId::NamedInteger("schema_table".into(), s_idx as u64))
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .justify_between()
                                                    .p_1()
                                                    .rounded_sm()
                                                    .bg(rgb(0x1e293b))
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(rgb(0x334155)))
                                                    .on_click(cx.listener({
                                                        let tbl_name = tbl_name.clone();
                                                        move |this, _event, _window, cx| {
                                                            let current = *this.state.tablas_expandidas.get(&tbl_name).unwrap_or(&false);
                                                            this.state.tablas_expandidas.insert(tbl_name.clone(), !current);
                                                            cx.notify();
                                                        }
                                                    }))
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_weight(FontWeight::BOLD)
                                                            .text_color(rgb(0x38bdf8))
                                                            .child(format!("{} {}", if is_expanded { "▼" } else { "▶" }, tbl_name)),
                                                    )
                                                    .child(
                                                        div()
                                                            .id(ElementId::NamedInteger("insert_table".into(), s_idx as u64))
                                                            .px_1()
                                                            .rounded_sm()
                                                            .bg(rgb(0x0f172a))
                                                            .text_xs()
                                                            .text_color(rgb(0x64748b))
                                                            .cursor_pointer()
                                                            .on_click(cx.listener({
                                                                let tbl_name = tbl_name.clone();
                                                                move |this, _event, _window, cx| {
                                                                    this.state.sql_buffer.push_str(&tbl_name);
                                                                    this.state.sql_buffer.push(' ');
                                                                    cx.notify();
                                                                }
                                                            }))
                                                            .child("+ Insertar"),
                                                    ),
                                            )
                                            .when(is_expanded, |this| {
                                                this.child(
                                                    div()
                                                        .pl_3()
                                                        .flex()
                                                        .flex_col()
                                                        .gap_1()
                                                        .children(cols.into_iter().enumerate().map(|(c_idx, col)| {
                                                            let col_name = col.name.clone();
                                                            div()
                                                                .id(ElementId::NamedInteger("col_item".into(), ((s_idx as u64) << 16) | (c_idx as u64)))
                                                                .flex()
                                                                .flex_row()
                                                                .items_center()
                                                                .justify_between()
                                                                .px_1()
                                                                .text_xs()
                                                                .cursor_pointer()
                                                                .hover(|s| s.text_color(rgb(0x38bdf8)))
                                                                .on_click(cx.listener({
                                                                    let col_name = col_name.clone();
                                                                    move |this, _event, _window, cx| {
                                                                        this.state.sql_buffer.push_str(&col_name);
                                                                        this.state.sql_buffer.push(' ');
                                                                        cx.notify();
                                                                    }
                                                                }))
                                                                .child(
                                                                    div()
                                                                        .text_color(if col.is_pk { rgb(0xfbbf24) } else { rgb(0x94a3b8) })
                                                                        .child(format!("• {}{}", col.name, if col.is_pk { " (PK)" } else { "" })),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_color(rgb(0x64748b))
                                                                        .child(col.data_type),
                                                                )
                                                        })),
                                                )
                                            })
                                    })),
                            ),
                    ),
            )
            // =========================================================================
            // 3. MODAL CLASIFICACIÓN / PUNTUACIÓN POR CATEGORÍAS (SIN EQUIPOS)
            // =========================================================================
            .when(self.state.mostrar_modal_ranking, |this| {
                this.child(
                    div()
                        .absolute()
                        .size_full()
                        .bg(rgba(0x000000d0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(620.0))
                                .p_6()
                                .rounded_xl()
                                .bg(rgb(0x0f172a))
                                .border_1()
                                .border_color(rgb(0x334155))
                                .flex()
                                .flex_col()
                                .gap_4()
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .gap_2()
                                                .child(div().text_lg().child("🏆"))
                                                .child(div().text_base().font_weight(FontWeight::BOLD).child("Resultados y Desempeño por Categorías")),
                                        )
                                        .child(
                                            div()
                                                .id("modal_close")
                                                .cursor_pointer()
                                                .text_sm()
                                                .text_color(rgb(0x94a3b8))
                                                .on_click(cx.listener(|this, _event, _window, cx| {
                                                    this.state.mostrar_modal_ranking = false;
                                                    cx.notify();
                                                }))
                                                .child("✕"),
                                        ),
                                )
                                .child(self.render_level_breakdown_table())
                                .child(
                                    div()
                                        .p_3()
                                        .rounded_md()
                                        .bg(rgb(0x1e293b))
                                        .flex()
                                        .flex_row()
                                        .justify_between()
                                        .items_center()
                                        .child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0x94a3b8))
                                                        .child(format!(
                                                            "Participante: {} · Aciertos: {}/{} · Errores: {}",
                                                            self.state.participante_nombre, aciertos, total_retos, errores
                                                        )),
                                                )
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_color(rgb(0x10b981))
                                                        .child(format!("RESULTADO ACTUAL: {} / {}", aciertos, total_retos)),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .id("modal_export")
                                                        .px_4()
                                                        .py_2()
                                                        .rounded_md()
                                                        .bg(rgb(0x059669))
                                                        .text_xs()
                                                        .font_weight(FontWeight::BOLD)
                                                        .cursor_pointer()
                                                        .hover(|s| s.bg(rgb(0x10b981)))
                                                        .on_click(cx.listener(|this, _event, _window, cx| {
                                                            this.state.exportar_resultados();
                                                            cx.notify();
                                                        }))
                                                        .child("💾 Exportar CSV y JSON"),
                                                ),
                                        ),
                                ),
                        ),
                )
            })
            // =========================================================================
            // 4. MODAL DE CONFIRMACIÓN DE EXPORTACIÓN (REUTILIZADO)
            // =========================================================================
            .when(self.state.mostrar_modal_exportacion, |this| {
                this.child(self.render_export_modal(cx))
            })
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1320.0), px(860.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("SQL Sprint Davante - Simulador Tema 7 (ASIR)".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        cx.open_window(options, |_window, cx| {
            cx.new(|cx| SimuladorApp::new(cx))
        })
        .unwrap();
    });
}
