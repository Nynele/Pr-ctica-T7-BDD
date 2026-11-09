use gpui::prelude::*;
use gpui::*;
use std::path::Path;
use std::time::Duration;

mod app_state;
mod db;
mod export;
mod models;
mod validator;

use app_state::SimuladorState;

struct SimuladorApp {
    state: SimuladorState,
    focus_handle: FocusHandle,
}

impl SimuladorApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

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
        }
    }

    fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = &event.keystroke.key;
        if key == "backspace" {
            self.state.sql_buffer.pop();
            cx.notify();
        } else if key == "enter" {
            if event.keystroke.modifiers.control {
                self.state.ejecutar_y_comprobar();
            } else {
                self.state.sql_buffer.push('\n');
            }
            cx.notify();
        } else if key == "space" {
            self.state.sql_buffer.push(' ');
            cx.notify();
        } else if key == "tab" {
            self.state.sql_buffer.push_str("    ");
            cx.notify();
        } else if key.len() == 1 {
            self.state.sql_buffer.push_str(key);
            cx.notify();
        }
    }
}

impl Render for SimuladorApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let reto_actual = self.state.reto_actual().cloned();
        let total_retos = self.state.retos.len();
        let actual_idx = self.state.reto_actual_idx;
        let aciertos = self.state.retos_superados.len();
        let intentos = self.state.historial_intentos.len();
        let puntos_actuales = self.state.puntos_actuales();
        let puntos_maximos = self.state.puntos_maximos();

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

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x0b0f19)) // Fondo general oscuro moderno
            .text_color(rgb(0xf1f5f9))
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                this.handle_key(event, cx);
            }))
            // =========================================================================
            // 1. CABECERA PRINCIPAL CON LOGO, TIEMPO GLOBAL Y SIN EQUIPOS
            // =========================================================================
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .h(px(56.0))
                    .px_6()
                    .bg(rgb(0x0f172a))
                    .border_b_1()
                    .border_color(rgb(0x1e293b))
                    // Lado izquierdo: Logotipo + Nombre del centro y app
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(
                                img(Path::new("logotipo.png"))
                                    .w(px(36.0))
                                    .h(px(36.0))
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
                                    ),
                            ),
                    )
                    // Lado derecho: Puntos, Temporizador Global, Participante y Acciones
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            // Puntuación acumulada
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x1e293b))
                                    .border_1()
                                    .border_color(rgb(0x334155))
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xfbbf24))
                                    .child(format!("⭐ {} / {} pts", puntos_actuales, puntos_maximos)),
                            )
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
                            // Cabecera del banco de retos
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
                                            .child(format!("BANCO DE RETOS ({}/30)", aciertos)),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0xfbbf24))
                                            .child(format!("{} pts", puntos_actuales)),
                                    ),
                            )
                            // Lista de retos con categorías y colores de dificultad
                            .child(
                                div()
                                    .id("retos_list")
                                    .flex()
                                    .flex_col()
                                    .overflow_y_scroll()
                                    .p_2()
                                    .gap_1()
                                    .children(self.state.retos.iter().enumerate().map(|(idx, r)| {
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

                                        let badge_col = rgb(r.color_hex());

                                        // Renderizado de separador de categoría si es el primer ejercicio del bloque
                                        let category_header = if idx == 0 || self.state.retos[idx - 1].nivel != r.nivel {
                                            Some(
                                                div()
                                                    .pt_2()
                                                    .pb_1()
                                                    .px_2()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(badge_col)
                                                    .child(format!("NIVEL {} · {} ({} pts)", r.nivel, r.categoria().to_uppercase(), r.puntos())),
                                            )
                                        } else {
                                            None
                                        };

                                        div()
                                            .flex()
                                            .flex_col()
                                            .children(category_header)
                                            .child(
                                                div()
                                                    .id(ElementId::NamedInteger("reto_item".into(), idx as u64))
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .justify_between()
                                                    .p_2()
                                                    .rounded_md()
                                                    .bg(bg_color)
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(rgb(0x334155)))
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
                                                            .child(div().text_xs().text_color(icon_color).child(icon))
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .font_weight(FontWeight::MEDIUM)
                                                                    .child(format!("{}: {}", r.id, r.titulo)),
                                                            ),
                                                    )
                                                    // Badge de dificultad y puntos con color temático
                                                    .child(
                                                        div()
                                                            .px_2()
                                                            .py_0p5()
                                                            .rounded_sm()
                                                            .bg(rgb(0x131d31))
                                                            .border_1()
                                                            .border_color(badge_col)
                                                            .text_xs()
                                                            .font_weight(FontWeight::BOLD)
                                                            .text_color(badge_col)
                                                            .child(format!("+{} pts", r.puntos())),
                                                    ),
                                            )
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
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap_3()
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
                                                    // Badge con categoría y puntos coloreados
                                                    .when_some(reto_actual.as_ref(), |this, r| {
                                                        let col = rgb(r.color_hex());
                                                        this.child(
                                                            div()
                                                                .px_2()
                                                                .py_1()
                                                                .rounded_md()
                                                                .bg(rgb(0x0f172a))
                                                                .border_1()
                                                                .border_color(col)
                                                                .text_xs()
                                                                .font_weight(FontWeight::BOLD)
                                                                .text_color(col)
                                                                .child(format!("{} · {} PTS", r.dificultad_label(), r.puntos())),
                                                        )
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
                                                    .flex()
                                                    .flex_row()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .id("btn_clear_sql")
                                                            .px_2()
                                                            .py_1()
                                                            .rounded_sm()
                                                            .bg(rgb(0x334155))
                                                            .text_xs()
                                                            .cursor_pointer()
                                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                                this.state.sql_buffer.clear();
                                                                cx.notify();
                                                            }))
                                                            .child("Limpiar"),
                                                    )
                                                    .child(
                                                        div()
                                                            .id("btn_canonical_solution")
                                                            .px_2()
                                                            .py_1()
                                                            .rounded_sm()
                                                            .bg(rgb(0x0284c7))
                                                            .text_xs()
                                                            .cursor_pointer()
                                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                                if let Some(r) = this.state.reto_actual() {
                                                                    this.state.sql_buffer = r.consulta_referencia.clone();
                                                                    cx.notify();
                                                                }
                                                            }))
                                                            .child("💡 Solución Canónica (Demo)"),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .p_3()
                                            .min_h(px(100.0))
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
                                            .child("Siguiente reto ⏭"),
                                    ),
                            )
                            // --- BANNER DE RETROALIMENTACIÓN ---
                            .when_some(self.state.feedback_banner.clone(), |this, (es_exito, msg)| {
                                let (bg_color, border_col, txt_col) = if es_exito {
                                    (rgb(0x064e3b), rgb(0x059669), rgb(0x6ee7b7))
                                } else {
                                    (rgb(0x451a03), rgb(0xd97706), rgb(0xfde68a))
                                };

                                this.child(
                                    div()
                                        .p_3()
                                        .rounded_md()
                                        .bg(bg_color)
                                        .border_1()
                                        .border_color(border_col)
                                        .text_sm()
                                        .text_color(txt_col)
                                        .child(msg),
                                )
                            })
                            // --- TABLA DE RESULTADOS DE LA CONSULTA ---
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
                                                    .child("RESULTADO DE LA CONSULTA"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x38bdf8))
                                                    .child(if let Some(ref q) = self.state.query_result {
                                                        format!("{:.3} ms | Total: {} filas", q.execution_time_ms, q.total_rows)
                                                    } else {
                                                        "Esperando ejecución...".to_string()
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("result_table")
                                            .max_h(px(260.0))
                                            .overflow_y_scroll()
                                            .p_2()
                                            .child(if let Some(ref q) = self.state.query_result {
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .size_full()
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
                            // Tarjeta de métricas con puntos y tiempo global
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
                                            .child(div().text_xs().text_color(rgb(0xcbd5e1)).child("Puntos totales:"))
                                            .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(0xfbbf24)).child(format!("{} / {} pts", puntos_actuales, puntos_maximos))),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .justify_between()
                                            .child(div().text_xs().text_color(rgb(0xcbd5e1)).child("Retos superados:"))
                                            .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(0x10b981)).child(format!("{}/30", aciertos))),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .justify_between()
                                            .child(div().text_xs().text_color(rgb(0xcbd5e1)).child("Intentos acumulados:"))
                                            .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(0x38bdf8)).child(format!("{}", intentos))),
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
                // Conteo por niveles
                let n1_ok = self.state.retos.iter().filter(|r| r.nivel == 1 && self.state.retos_superados.contains(&r.id)).count();
                let n2_ok = self.state.retos.iter().filter(|r| r.nivel == 2 && self.state.retos_superados.contains(&r.id)).count();
                let n3_ok = self.state.retos.iter().filter(|r| r.nivel == 3 && self.state.retos_superados.contains(&r.id)).count();
                let n4_ok = self.state.retos.iter().filter(|r| r.nivel == 4 && self.state.retos_superados.contains(&r.id)).count();
                let n5_ok = self.state.retos.iter().filter(|r| r.nivel == 5 && self.state.retos_superados.contains(&r.id)).count();

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
                                .w(px(580.0))
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
                                // Tabla de niveles, puntos y categorías
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_2()
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .p_2()
                                                .bg(rgb(0x1e293b))
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .child(div().w(px(70.0)).child("Nivel"))
                                                .child(div().w(px(220.0)).child("Categoría"))
                                                .child(div().w(px(90.0)).child("Superados"))
                                                .child(div().w(px(70.0)).child("Puntos"))
                                                .child(div().w(px(80.0)).child("Estado")),
                                        )
                                        // Nivel 1
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .p_2()
                                                .bg(rgb(0x131d31))
                                                .text_xs()
                                                .text_color(rgb(0x10b981))
                                                .child(div().w(px(70.0)).child("Nivel 1"))
                                                .child(div().w(px(220.0)).text_color(rgb(0xe2e8f0)).child("Selección y Proyección"))
                                                .child(div().w(px(90.0)).child(format!("{}/6", n1_ok)))
                                                .child(div().w(px(70.0)).font_weight(FontWeight::BOLD).child(format!("{} pts", n1_ok * 10)))
                                                .child(div().w(px(80.0)).child(if n1_ok == 6 { "Completado" } else { "En curso" })),
                                        )
                                        // Nivel 2
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .p_2()
                                                .bg(rgb(0x0f172a))
                                                .text_xs()
                                                .text_color(rgb(0x06b6d4))
                                                .child(div().w(px(70.0)).child("Nivel 2"))
                                                .child(div().w(px(220.0)).text_color(rgb(0xe2e8f0)).child("Filtrado con WHERE"))
                                                .child(div().w(px(90.0)).child(format!("{}/6", n2_ok)))
                                                .child(div().w(px(70.0)).font_weight(FontWeight::BOLD).child(format!("{} pts", n2_ok * 20)))
                                                .child(div().w(px(80.0)).child(if n2_ok == 6 { "Completado" } else { "En curso" })),
                                        )
                                        // Nivel 3
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .p_2()
                                                .bg(rgb(0x131d31))
                                                .text_xs()
                                                .text_color(rgb(0x8b5cf6))
                                                .child(div().w(px(70.0)).child("Nivel 3"))
                                                .child(div().w(px(220.0)).text_color(rgb(0xe2e8f0)).child("Ordenación y Límites"))
                                                .child(div().w(px(90.0)).child(format!("{}/6", n3_ok)))
                                                .child(div().w(px(70.0)).font_weight(FontWeight::BOLD).child(format!("{} pts", n3_ok * 30)))
                                                .child(div().w(px(80.0)).child(if n3_ok == 6 { "Completado" } else { "En curso" })),
                                        )
                                        // Nivel 4
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .p_2()
                                                .bg(rgb(0x0f172a))
                                                .text_xs()
                                                .text_color(rgb(0xf59e0b))
                                                .child(div().w(px(70.0)).child("Nivel 4"))
                                                .child(div().w(px(220.0)).text_color(rgb(0xe2e8f0)).child("Resumen y Agregación"))
                                                .child(div().w(px(90.0)).child(format!("{}/6", n4_ok)))
                                                .child(div().w(px(70.0)).font_weight(FontWeight::BOLD).child(format!("{} pts", n4_ok * 40)))
                                                .child(div().w(px(80.0)).child(if n4_ok == 6 { "Completado" } else { "En curso" })),
                                        )
                                        // Nivel 5
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .p_2()
                                                .bg(rgb(0x131d31))
                                                .text_xs()
                                                .text_color(rgb(0xef4444))
                                                .child(div().w(px(70.0)).child("Nivel 5"))
                                                .child(div().w(px(220.0)).text_color(rgb(0xe2e8f0)).child("Agrupación Avanzada (HAVING)"))
                                                .child(div().w(px(90.0)).child(format!("{}/6", n5_ok)))
                                                .child(div().w(px(70.0)).font_weight(FontWeight::BOLD).child(format!("{} pts", n5_ok * 50)))
                                                .child(div().w(px(80.0)).child(if n5_ok == 6 { "Completado" } else { "En curso" })),
                                        ),
                                )
                                // Resumen total y acciones
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
                                                .child(div().text_xs().text_color(rgb(0x94a3b8)).child(format!("Participante: {}", self.state.participante_nombre)))
                                                .child(div().text_sm().font_weight(FontWeight::BOLD).text_color(rgb(0xfbbf24)).child(format!("PUNTUACIÓN TOTAL: {} / {} PTS", puntos_actuales, puntos_maximos))),
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
            // 4. MODAL DE CONFIRMACIÓN DE EXPORTACIÓN (100% FUNCIONAL)
            // =========================================================================
            .when(self.state.mostrar_modal_exportacion, |this| {
                let json_path = self.state.ruta_export_json.clone().unwrap_or_else(|| "exports/resultados_sprint_sql.json".to_string());
                let csv_path = self.state.ruta_export_csv.clone().unwrap_or_else(|| "exports/resultados_sprint_sql.csv".to_string());

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
                                .w(px(560.0))
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
                                                .child(div().text_base().font_weight(FontWeight::BOLD).text_color(rgb(0x10b981)).child("¡Resultados Exportados con Éxito!")),
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
                                                .child(format!("Sesión: #{} · Participante: {}", self.state.sesion_id, self.state.participante_nombre)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0xcbd5e1))
                                                .child(format!("• Puntuación conseguida: {} / {} pts", puntos_actuales, puntos_maximos)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0xcbd5e1))
                                                .child(format!("• Retos superados: {} de 30", aciertos)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0xcbd5e1))
                                                .child(format!("• Intentos registrados: {}", intentos)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0xcbd5e1))
                                                .child(format!("• Tiempo total invertido: {:02}:{:02}", self.state.tiempo_total_segundos / 60, self.state.tiempo_total_segundos % 60)),
                                        ),
                                )
                                // Rutas de los archivos exportados
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_2()
                                        .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(rgb(0x94a3b8)).child("ARCHIVOS GENERADOS:"))
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
                                // Botones de acción
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
                        ),
                )
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
