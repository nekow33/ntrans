//! egui 主界面。

use std::sync::mpsc::{Receiver, Sender, channel};

use egui::{Color32, RichText};

use crate::app_settings::Settings;
use crate::history::History;
use crate::i18n::{self, Texts, UiLang};
use crate::model::{LANGS, Lang, TranslateOut, TranslateRequest, find as find_lang};
use crate::providers::{Engine, translate};
use crate::tray;
use crate::ui_msg::UiMsg;

pub struct TranslatorApp {
    settings: Settings,
    history: History,

    source: String,
    output: String,
    from_idx: usize,
    to_idx: usize,
    engine: Engine,

    busy: bool,
    pending_seq: u64,
    last_out: Option<TranslateOut>,
    error: Option<String>,

    tx: Sender<UiMsg>,
    rx: Receiver<UiMsg>,
    runtime: tokio::runtime::Runtime,

    show_settings: bool,
    show_history: bool,
    hidden: bool,
    quitting: bool,
    tray: Option<tray::Handle>,
}

impl TranslatorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::install_cjk(&cc.egui_ctx);
        // 浅色主题。
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        cc.egui_ctx.set_theme(egui::ThemePreference::Light);

        let settings = Settings::load();
        let history = History::load(settings.history_capacity);
        let (tx, rx) = channel();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("无法创建 tokio 运行时");

        let tray = tray::Handle::create(&cc.egui_ctx);

        Self {
            settings,
            history,
            source: String::new(),
            output: String::new(),
            from_idx: 0,
            to_idx: 1,
            engine: Engine::Auto,
            busy: false,
            pending_seq: 0,
            last_out: None,
            error: None,
            tx,
            rx,
            runtime,
            show_settings: false,
            show_history: false,
            hidden: false,
            quitting: false,
            tray,
        }
    }

    fn texts(&self) -> &'static Texts {
        self.settings.ui_lang.texts()
    }

    fn current_langs(&self) -> (Lang, Lang) {
        let from = LANGS[self.from_idx.min(LANGS.len() - 1)];
        let to = LANGS[self.to_idx.min(LANGS.len() - 1)];
        (from, to)
    }

    fn lang_label(&self, code: &str) -> String {
        find_lang(code)
            .map(|l| l.name(self.settings.ui_lang).to_owned())
            .unwrap_or_else(|| code.to_owned())
    }

    // ---------------- 翻译与消息 ----------------

    /// 智能方向：开启时源语言为“自动”且目标为 中/英 之一时，
    /// 根据输入内容实时把目标切到 英文（中文输入）或 中文（其他）。
    fn auto_pick_target(&mut self) {
        if !self.settings.auto_direction {
            return;
        }
        let source_is_auto = LANGS[self.from_idx.min(LANGS.len() - 1)].code == "auto";
        let target_is_default =
            matches!(LANGS[self.to_idx.min(LANGS.len() - 1)].code, "en" | "zh-CN");
        if !(source_is_auto && target_is_default) {
            return;
        }
        let want_en = crate::model::is_chinese(&self.source);
        let code = if want_en { "en" } else { "zh-CN" };
        if let Some(pos) = LANGS.iter().position(|l| l.code == code) {
            self.to_idx = pos;
        }
    }

    fn start_translate(&mut self) {
        let text = self.source.trim().to_owned();
        if text.is_empty() {
            let lang = self.settings.ui_lang;
            self.error = Some(i18n::err_input_empty(lang).to_owned());
            return;
        }
        if self.busy {
            return;
        }
        self.auto_pick_target();
        let (from, mut to) = self.current_langs();
        if !to.can_be_target {
            to = LANGS[1];
            self.to_idx = 1;
        }
        let req = TranslateRequest {
            text,
            from: from.code.to_owned(),
            to: to.code.to_owned(),
        };
        let engine = self.engine;
        let lang = self.settings.ui_lang;
        self.pending_seq = self.pending_seq.wrapping_add(1);
        let seq = self.pending_seq;
        self.busy = true;
        self.error = None;
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            let res = translate(&req, engine, lang).await;
            let _ = tx.send(UiMsg {
                seq,
                result: res.map_err(|e| e.to_localized(lang)),
            });
        });
    }

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            if msg.seq != self.pending_seq {
                continue; // 过期结果
            }
            self.busy = false;
            match msg.result {
                Ok(out) => {
                    let (_, to) = self.current_langs();
                    self.output = out.text.clone();
                    self.error = None;
                    self.last_out = Some(out.clone());
                    let source = self.source.trim().to_owned();
                    self.history.add(
                        source,
                        out.text.clone(),
                        out.detected_from.clone(),
                        to.code.to_owned(),
                        out.engine.to_owned(),
                    );
                    self.history.flush();
                }
                Err(e) => {
                    self.error = Some(e);
                }
            }
        }
    }

    // ---------------- 窗口/托盘 ----------------

    fn hide_window(&mut self, ctx: &egui::Context) {
        self.hidden = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    fn show_window(&mut self, ctx: &egui::Context) {
        self.hidden = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn toggle_window(&mut self, ctx: &egui::Context) {
        if self.hidden {
            self.show_window(ctx);
        } else {
            self.hide_window(ctx);
        }
    }

    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if !close_requested {
            return;
        }
        if self.quitting {
            return; // 允许真正退出
        }
        if self.tray.is_some() && self.settings.hide_on_close {
            self.hidden = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    fn copy_text(&self, ctx: &egui::Context, s: &str) {
        ctx.copy_text(s.to_owned());
    }

    fn status_text(&self) -> String {
        let tr = self.texts();
        match (&self.error, &self.last_out) {
            (Some(e), _) => e.clone(),
            (None, Some(out)) => {
                let detected = self.lang_label(&out.detected_from);
                match self.settings.ui_lang {
                    UiLang::En => {
                        format!("Done · Source: {detected} · Engine: {}", out.engine)
                    }
                    UiLang::Zh => {
                        format!("翻译完成  源语言: {detected}   引擎: {}", out.engine)
                    }
                }
            }
            (None, None) => tr.ready.to_owned(),
        }
    }

    fn status_ui(&mut self, ui: &mut egui::Ui) {
        let status = self.status_text();
        let color = if self.error.is_some() {
            Color32::from_rgb(0xe0, 0x60, 0x60)
        } else if self.last_out.is_some() {
            Color32::from_rgb(0x50, 0xa0, 0x60)
        } else {
            ui.visuals().weak_text_color()
        };
        ui.label(RichText::new(status).color(color));
    }

    // ---------------- UI 子区域 ----------------

    /// 顶部菜单栏：设置 / 历史（左对齐）。
    fn menu_bar_ui(&mut self, ui: &mut egui::Ui) {
        let tr = self.texts();
        ui.horizontal(|ui| {
            if ui.selectable_label(self.show_settings, tr.settings).clicked() {
                self.show_settings = !self.show_settings;
            }
            if ui.selectable_label(self.show_history, tr.history).clicked() {
                self.show_history = !self.show_history;
            }
        });
    }

    /// 菜单栏下方的控制条：引擎 / 语言 / 翻译方向。
    fn lang_row_ui(&mut self, ui: &mut egui::Ui) {
        let tr = self.texts();
        let ui_lang = self.settings.ui_lang;
        ui.horizontal_wrapped(|ui| {
            ui.label(tr.engine);
            egui::ComboBox::from_id_salt("engine")
                .selected_text(i18n::engine_display(self.engine, ui_lang))
                .show_ui(ui, |ui| {
                    for e in Engine::ALL {
                        let name = i18n::engine_display(e, ui_lang);
                        ui.selectable_value(&mut self.engine, e, name);
                    }
                });

            ui.separator();
            ui.label(tr.from);
            egui::ComboBox::from_id_salt("from_lang")
                .selected_text(LANGS[self.from_idx].name(ui_lang))
                .show_ui(ui, |ui| {
                    for (i, lang) in LANGS.iter().enumerate() {
                        let name = lang.name(ui_lang);
                        ui.selectable_value(&mut self.from_idx, i, name);
                    }
                });

            if ui
                .add(egui::Button::new(tr.swap).frame(false))
                .on_hover_text(tr.swap)
                .clicked()
            {
                let tmp = self.from_idx;
                self.from_idx = self.to_idx;
                self.to_idx = if LANGS[tmp].can_be_target { tmp } else { 1 };
            }

            ui.label(tr.to);
            let targets: Vec<(usize, &Lang)> = LANGS
                .iter()
                .enumerate()
                .filter(|(_, l)| l.can_be_target)
                .collect();
            egui::ComboBox::from_id_salt("to_lang")
                .selected_text(LANGS[self.to_idx].name(ui_lang))
                .show_ui(ui, |ui| {
                    for (i, lang) in targets {
                        let name = lang.name(ui_lang);
                        ui.selectable_value(&mut self.to_idx, i, name);
                    }
                });

            ui.add_space(8.0);
            let label = if self.busy {
                tr.translating
            } else {
                tr.translate
            };
            if ui
                .add_enabled(!self.busy, egui::Button::new(label))
                .on_hover_text(tr.translate_shortcut)
                .clicked()
            {
                self.start_translate();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .checkbox(&mut self.settings.auto_direction, tr.auto_direction)
                    .on_hover_text(tr.auto_direction_hint)
                    .changed()
                {
                    self.settings.save();
                    self.auto_pick_target();
                }
            });
        });
    }

    /// 绘制一个填满指定高度的多行文本框（内容多时内部滚动）。
    #[allow(clippy::too_many_arguments)]
    fn edit_box(
        ui: &mut egui::Ui,
        id_salt: &str,
        text: &mut String,
        height: f32,
        hint: &str,
        monospace: bool,
    ) {
        let width = ui.available_width();
        let height = height.max(40.0);
        let (_id, rect) = ui.allocate_space(egui::vec2(width, height));

        // 按高度换算出需要的行数，让编辑框一上来就铺满分配区域。
        let text_style = if monospace {
            egui::TextStyle::Monospace
        } else {
            egui::TextStyle::Body
        };
        let font_id = ui
            .style()
            .text_styles
            .get(&text_style)
            .cloned()
            .unwrap_or_else(|| egui::FontId::proportional(14.0));
        let row_h = ui.ctx().fonts_mut(|f| f.row_height(&font_id)).max(4.0);
        let rows = (((height - 22.0) / row_h).floor()).max(3.0) as usize;

        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down_justified(egui::Align::Min)),
        );
        egui::ScrollArea::vertical()
            .id_salt(id_salt)
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.set_min_height(height - 6.0);
                let mut te = egui::TextEdit::multiline(text)
                    .desired_width(f32::INFINITY)
                    .desired_rows(rows)
                    .hint_text(hint)
                    .margin(egui::Margin::symmetric(8, 6));
                if monospace {
                    te = te.font(egui::TextStyle::Monospace);
                }
                ui.add(te);
            });
    }

    fn source_ui(&mut self, ui: &mut egui::Ui, height: f32) {
        let tr = self.texts();
        ui.horizontal(|ui| {
            ui.strong(tr.source);
            if !self.source.is_empty() && ui.small_button(tr.clear).clicked() {
                self.source.clear();
            }
        });
        let mut text = self.source.clone();
        Self::edit_box(ui, "source_edit", &mut text, height, tr.input_hint, false);
        self.source = text;
        self.auto_pick_target();
    }

    fn result_ui(&mut self, ui: &mut egui::Ui, height: f32) {
        let tr = self.texts();
        ui.horizontal(|ui| {
            ui.strong(tr.target);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.busy {
                    ui.add(egui::Spinner::new().size(13.0));
                }
                if !self.output.is_empty() {
                    if ui.small_button(tr.copy).clicked() {
                        self.copy_text(ui.ctx(), &self.output);
                    }
                    if ui.small_button(tr.use_as_source).clicked() {
                        self.source = self.output.clone();
                    }
                }
            });
        });
        let mut text = self.output.clone();
        Self::edit_box(ui, "result_edit", &mut text, height, tr.output_hint, true);
        self.output = text;
    }

    /// 主区域：原文/译文两个编辑框，按比例填满剩余空间。
    fn central_body(&mut self, ui: &mut egui::Ui) {
        let total = ui.available_height();
        let source_h = (total * 0.42).clamp(80.0, 600.0);

        self.source_ui(ui, source_h);
        ui.add_space(6.0);

        let result_h = ui.available_height().max(60.0);
        self.result_ui(ui, result_h);
    }

    fn history_ui(&mut self, ui: &mut egui::Ui) {
        let tr = self.texts();
        ui.horizontal(|ui| {
            let title = if self.settings.ui_lang == UiLang::En {
                format!("{} ({})", tr.history, self.history.entries.len())
            } else {
                tr.history_title
                    .replace("{n}", &self.history.entries.len().to_string())
            };
            ui.strong(title);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(tr.clear_all).clicked() {
                    self.history.clear();
                    self.history.flush();
                }
            });
        });

        let snapshot = self.history.entries.clone();
        if snapshot.is_empty() {
            ui.label(RichText::new(tr.history_empty).weak());
            return;
        }

        let mut to_remove: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt("history_scroll")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (i, entry) in snapshot.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&entry.at).weak());
                        ui.label(format!(
                            "{} → {}",
                            self.lang_label(&entry.from),
                            self.lang_label(&entry.to)
                        ));
                        ui.label(RichText::new(format!("[{}]", entry.engine)).weak());
                    });
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&entry.source).color(Color32::from_rgb(0x80, 0x90, 0xa0)),
                        );
                        ui.label(RichText::new("→").weak());
                        ui.label(
                            RichText::new(&entry.translation)
                                .color(Color32::from_rgb(0x90, 0xb0, 0xe0)),
                        );
                    });
                    ui.horizontal(|ui| {
                        let (source, translation, from, to) = (
                            entry.source.clone(),
                            entry.translation.clone(),
                            entry.from.clone(),
                            entry.to.clone(),
                        );
                        if ui.small_button(tr.fill_back).clicked() {
                            self.source = source;
                            self.output = translation.clone();
                            let pos_of = |code: &str| {
                                find_lang(code)
                                    .and_then(|l| LANGS.iter().position(|x| x.code == l.code))
                            };
                            if let Some(p) = pos_of(&from) {
                                self.from_idx = p;
                            }
                            if let Some(p) = pos_of(&to) {
                                self.to_idx = p;
                            }
                            self.last_out = None;
                            self.error = None;
                        }
                        if ui.small_button(tr.copy_translation).clicked() {
                            self.copy_text(ui.ctx(), &translation);
                        }
                        if ui.small_button(tr.delete).clicked() {
                            to_remove = Some(entry.id.clone());
                        }
                    });
                    if i + 1 < snapshot.len() {
                        ui.separator();
                    }
                }
            });
        if let Some(id) = to_remove {
            self.history.remove(&id);
            self.history.flush();
        }
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let tr = self.texts();
        let ui_lang = self.settings.ui_lang;
        let tray_ok = self.tray.is_some();
        let mut hide = self.settings.hide_on_close;

        ui.add_space(4.0);
        ui.label(RichText::new(tr.section_language).strong());
        egui::ComboBox::from_id_salt("ui_lang")
            .selected_text(ui_lang.label())
            .show_ui(ui, |ui| {
                for lang in [UiLang::En, UiLang::Zh] {
                    if ui
                        .selectable_value(&mut self.settings.ui_lang, lang, lang.label())
                        .changed()
                    {
                        self.settings.save();
                    }
                }
            });
        ui.label(RichText::new(tr.language_note).weak().small());

        ui.add_space(8.0);
        ui.label(RichText::new(tr.section_general).strong());
        if ui
            .checkbox(&mut self.settings.auto_direction, tr.auto_direction)
            .on_hover_text(tr.auto_direction_hint)
            .changed()
        {
            self.settings.save();
            self.auto_pick_target();
        }

        ui.add_space(8.0);
        ui.label(RichText::new(tr.section_tray).strong());
        ui.add_enabled_ui(tray_ok, |ui| {
            ui.checkbox(&mut hide, tr.hide_on_close);
        });
        if !tray_ok {
            ui.label(RichText::new(tr.no_tray_note).weak().italics());
        }

        ui.add_space(8.0);
        ui.label(RichText::new(tr.section_history).strong());
        ui.horizontal(|ui| {
            ui.label(tr.keep_max);
            let mut cap = self.settings.history_capacity as i64;
            let resp = ui.add(
                egui::DragValue::new(&mut cap)
                    .range(1..=10_000)
                    .speed(10)
                    .suffix(format!(" {}", tr.entries_suffix)),
            );
            if resp.changed() {
                self.settings.history_capacity = cap.max(1) as usize;
                self.history.set_capacity(self.settings.history_capacity);
                self.settings.save();
                self.history.flush();
            }
        });

        ui.add_space(10.0);
        ui.label(RichText::new(tr.engine_note).weak());

        if hide != self.settings.hide_on_close {
            self.settings.hide_on_close = hide;
            self.settings.save();
        }
    }

    fn run_ui(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        let bg = root.visuals().panel_fill;
        let frame = |margin: egui::Margin| egui::Frame::default().fill(bg).inner_margin(margin);
        egui::Panel::top("menu_bar")
            .frame(frame(egui::Margin::symmetric(12, 5)))
            .show(root, |ui| {
                self.menu_bar_ui(ui);
            });

        egui::Panel::top("lang_bar")
            .frame(frame(egui::Margin::symmetric(10, 5)))
            .show(root, |ui| {
                self.lang_row_ui(ui);
            });

        egui::Panel::bottom("status")
            .resizable(false)
            .frame(frame(egui::Margin::symmetric(12, 4)))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    self.status_ui(ui);
                    if self.busy {
                        ui.add(egui::Spinner::new().size(14.0));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(self.texts().translate_shortcut)
                                .weak()
                                .small(),
                        );
                    });
                });
            });

        if self.show_history {
            egui::Panel::bottom("history_panel")
                .resizable(true)
                .default_size(200.0)
                .min_size(60.0)
                .frame(frame(egui::Margin::symmetric(10, 4)))
                .show(root, |ui| {
                    self.history_ui(ui);
                });
        }

        egui::CentralPanel::default()
            .frame(frame(egui::Margin::symmetric(12, 6)))
            .show(root, |ui| {
                self.central_body(ui);
            });

        if self.show_settings {
            let mut open = true;
            egui::Window::new(self.texts().settings_window)
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.set_width(360.0);
                    self.settings_ui(ui);
                });
            if !open {
                self.show_settings = false;
            }
        }
    }

    fn pump_tray(&mut self, ctx: &egui::Context) {
        for cmd in self.tray.as_ref().map(|t| t.pump()).unwrap_or_default() {
            match cmd {
                tray::Command::Toggle => self.toggle_window(ctx),
                tray::Command::Settings => {
                    self.show_settings = true;
                    self.show_window(ctx);
                }
                tray::Command::Quit => self.quit(ctx),
            }
        }
    }
}

impl Drop for TranslatorApp {
    fn drop(&mut self) {
        self.history.flush();
        self.settings.save();
    }
}

impl eframe::App for TranslatorApp {
    /// 每帧在绘制前调用（窗口隐藏时也可被唤醒）。
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_messages();
        if self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
        self.pump_tray(ctx);
    }

    /// 渲染主窗口内容。
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_close_request(&ctx);
        self.run_ui(ui, &ctx);

        // Ctrl/Cmd + Enter 翻译快捷键。
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter)) {
            self.start_translate();
        }
    }
}

/// 启动图形界面（`NativeOptions` 集中于此，便于 GUI/CLI 拆分的二进制共用）。
pub fn run_gui() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ntrans")
            .with_inner_size([900.0, 680.0])
            .with_min_inner_size([700.0, 520.0])
            .with_icon(crate::icon::window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "ntrans",
        options,
        Box::new(|cc| Ok(Box::new(TranslatorApp::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_for_test() -> TranslatorApp {
        let (tx, rx) = channel();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        TranslatorApp {
            settings: Settings::default(),
            history: History::fresh(20),
            source: "hello".into(),
            output: String::new(),
            from_idx: 0,
            to_idx: 1,
            engine: Engine::MyMemory,
            busy: false,
            pending_seq: 0,
            last_out: None,
            error: None,
            tx,
            rx,
            runtime,
            show_settings: false,
            show_history: false,
            hidden: false,
            quitting: false,
            tray: None,
        }
    }

    fn raw_input() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(960.0, 700.0),
            )),
            ..Default::default()
        }
    }

    #[test]
    fn ui_renders_multiple_frames() {
        let ctx = egui::Context::default();
        let _ = crate::fonts::install_cjk(&ctx);
        let mut app = app_for_test();
        app.show_history = true;
        app.history.add(
            "hello".into(),
            "你好".into(),
            "en".into(),
            "zh-CN".into(),
            "mymemory".into(),
        );

        for _ in 0..6 {
            let mut out = ctx.run_ui(raw_input(), |ui| {
                let c = ui.ctx().clone();
                app.run_ui(ui, &c);
            });
            out.textures_delta.clear();
        }
        assert_eq!(app.history.entries.len(), 1);
    }

    #[test]
    fn settings_window_renders() {
        let ctx = egui::Context::default();
        let _ = crate::fonts::install_cjk(&ctx);
        let mut app = app_for_test();
        app.show_settings = true;
        let mut out = ctx.run_ui(raw_input(), |ui| {
            let c = ui.ctx().clone();
            app.run_ui(ui, &c);
        });
        out.textures_delta.clear();
        app.show_settings = true;
    }

    #[test]
    fn ui_renders_in_chinese() {
        let ctx = egui::Context::default();
        let _ = crate::fonts::install_cjk(&ctx);
        let mut app = app_for_test();
        app.settings.ui_lang = UiLang::Zh;
        app.show_settings = true;
        for _ in 0..4 {
            let mut out = ctx.run_ui(raw_input(), |ui| {
                let c = ui.ctx().clone();
                app.run_ui(ui, &c);
            });
            out.textures_delta.clear();
        }
        assert_eq!(app.settings.ui_lang, UiLang::Zh);
    }

    #[test]
    fn drain_applies_matching_result() {
        let mut app = app_for_test();
        app.pending_seq = 7;
        app.tx
            .send(UiMsg {
                seq: 7,
                result: Ok(TranslateOut {
                    text: "你好，世界".into(),
                    detected_from: "en".into(),
                    engine: "MyMemory",
                }),
            })
            .unwrap();
        app.drain_messages();
        assert_eq!(app.output, "你好，世界");
        assert_eq!(app.history.entries.len(), 1);
        assert!(!app.busy);
    }

    #[test]
    fn stale_result_ignored() {
        let mut app = app_for_test();
        app.pending_seq = 8;
        app.tx
            .send(UiMsg {
                seq: 3,
                result: Err("stale".into()),
            })
            .unwrap();
        app.drain_messages();
        assert!(app.output.is_empty());
        assert!(app.history.entries.is_empty());
    }

    #[test]
    fn translate_empty_source_sets_error() {
        let mut app = app_for_test();
        app.source.clear();
        app.start_translate();
        assert!(app.error.is_some());
        assert!(!app.busy);
    }

    #[test]
    fn auto_direction_picks_target() {
        let mut app = app_for_test();
        app.from_idx = 0; // auto
        app.to_idx = 1; // zh-CN

        let en_idx = LANGS.iter().position(|l| l.code == "en").unwrap();
        let zh_idx = LANGS.iter().position(|l| l.code == "zh-CN").unwrap();

        app.source = "你好世界".into();
        app.auto_pick_target();
        assert_eq!(app.to_idx, en_idx, "中文输入应自动选英文");

        app.source = "hello world".into();
        app.auto_pick_target();
        assert_eq!(app.to_idx, zh_idx, "非中文输入应自动选中文");
    }

    #[test]
    fn auto_direction_respects_manual_target() {
        let mut app = app_for_test();
        app.from_idx = 0; // auto
        let ja_idx = LANGS.iter().position(|l| l.code == "ja").unwrap();
        app.to_idx = ja_idx;
        app.source = "你好".into();
        app.auto_pick_target();
        assert_eq!(app.to_idx, ja_idx, "手动选择的日语目标不应被覆盖");
    }

    #[test]
    fn auto_direction_off_keeps_target() {
        let mut app = app_for_test();
        app.settings.auto_direction = false;
        app.from_idx = 0;
        app.to_idx = 1;
        let zh_idx = 1;
        app.source = "hello world".into();
        app.auto_pick_target();
        assert_eq!(app.to_idx, zh_idx);
    }

    #[test]
    fn texts_localized() {
        assert_eq!(UiLang::En.texts().translate, "Translate");
        assert_eq!(UiLang::Zh.texts().translate, "翻译");
        assert_eq!(LANGS[2].name(UiLang::En), "English");
        assert_eq!(LANGS[2].name(UiLang::Zh), "英语");
    }
}
