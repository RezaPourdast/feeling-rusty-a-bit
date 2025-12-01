//! Application state, egui integration, and UI rendering.

use ping;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{self, Vec2};

use crate::domain::{AppState, DnsOperation, DnsProvider, DnsState, OperationResult};
use crate::system::{
    clear_dns_with_result, get_active_adapter, get_current_dns, set_dns_with_result,
};

/// Main application container used by eframe.
#[derive(Default)]
pub struct MyApp {
    adapter: Option<String>,
    dns: Vec<String>,
    app_state: AppState,
    selected_provider: DnsProvider,
    dns_state: DnsState,
    custom_primary: [String; 4],
    custom_secondary: [String; 4],
    operation_sender: Option<mpsc::Sender<OperationResult>>,
    operation_receiver: Option<mpsc::Receiver<OperationResult>>,
    show_second_window: bool,
    ping_value: f64,
    ping_sender: Option<mpsc::Sender<f64>>,
    ping_receiver: Option<mpsc::Receiver<f64>>,
}

// When the title-bar ping button is clicked we set this flag.
// `update()` will pick it up and start the ping thread / open the window.
static PING_REQUEST: AtomicBool = AtomicBool::new(false);

impl MyApp {
    pub fn new() -> Self {
        // Create app from defaults so we don't repeat many fields
        let app = Self {
            dns_state: DnsState::None,
            // don't create ping thread here — only when secondary window is opened
            ping_value: 0.0,
            ping_sender: None,
            ping_receiver: None,
            ..Default::default()
        };

        app
    }
}

impl eframe::App for MyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(receiver) = &self.operation_receiver {
            if let Ok(result) = receiver.try_recv() {
                self.handle_operation_result(result);
                self.operation_receiver = None;
                self.operation_sender = None;
                ctx.request_repaint();
            } else if matches!(self.app_state, AppState::Processing) {
                ctx.request_repaint();
            }
        }

        // New: check for ping updates (update UI only when a new ping arrives)
        if let Some(ping_rx) = &self.ping_receiver {
            if let Ok(ping) = ping_rx.try_recv() {
                self.ping_value = ping;
                ctx.request_repaint();
            }
        }

        custom_window_frame(ctx, "🚀 DNS SETTER", |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);

                ui.group(|ui| {
                    self.render_status_section(ui);
                    self.render_app_state(ui);
                });

                ui.add_space(30.0);

                ui.heading("🌐 Select DNS Provider");
                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    ui.add_space(140.0);
                    self.render_provider_selection(ui);
                });

                if matches!(self.selected_provider, DnsProvider::Custom { .. }) {
                    ui.add_space(20.0);
                    ui.group(|ui| {
                        ui.heading("📝 Custom DNS Settings");
                        ui.add_space(10.0);

                        Self::render_ip_input(ui, &mut self.custom_primary, "Primary DNS");

                        ui.add_space(5.0);

                        Self::render_ip_input(ui, &mut self.custom_secondary, "Secondary DNS");

                        ui.add_space(10.0);

                        if ui.button("Clear").clicked() {
                            self.custom_primary =
                                [String::new(), String::new(), String::new(), String::new()];
                            self.custom_secondary =
                                [String::new(), String::new(), String::new(), String::new()];
                        }
                    });
                }

                ui.add_space(40.0);

                ui.horizontal(|ui| {
                    ui.add_space(55.0);
                    self.render_action_buttons(ui);
                });

                ui.add_space(70.0);
            });
        });

        // If the title-bar ping button was clicked, start the ping thread / open the window.
        if PING_REQUEST.swap(false, Ordering::SeqCst) {
            if self.ping_sender.is_none() {
                let (tx, rx) = mpsc::channel::<f64>();
                self.ping_sender = Some(tx.clone());
                self.ping_receiver = Some(rx);

                thread::spawn(move || loop {
                    let value = get_ping();
                    if tx.send(value).is_err() {
                        break;
                    }
                    thread::sleep(Duration::from_secs(1));
                });
            }
            self.show_second_window = true;
        }

        self.render_secondary_viewport(ctx);
        ctx.request_repaint_after(Duration::from_millis(1000));
    }
}

impl MyApp {
    fn render_ip_input(ui: &mut egui::Ui, octets: &mut [String; 4], label: &str) {
        ui.horizontal(|ui| {
            ui.label(format!("{}: ", label));

            for (i, octet) in octets.iter_mut().enumerate() {
                let response = ui.add_sized(
                    Vec2::new(40.0, 20.0),
                    egui::TextEdit::singleline(octet)
                        .desired_width(40.0)
                        .char_limit(3),
                );

                if response.changed() {
                    *octet = octet.chars().filter(|c| c.is_ascii_digit()).collect();

                    if octet.len() == 3 && i < 3 {}
                }

                if i < 3 {
                    ui.label(egui::RichText::new(".").size(16.0));
                }
            }
        });
    }

    fn render_status_section(&self, ui: &mut egui::Ui) {
        ui.heading("📊 Current Status");

        match &self.dns_state {
            DnsState::Static(servers) => {
                ui.colored_label(egui::Color32::GREEN, "🔒 Static DNS Configuration");
                let fallback = String::from("None");
                let primary = servers.first().unwrap_or(&fallback);
                ui.label(format!("Primary: {}", primary));
            }
            DnsState::Dhcp => {
                ui.colored_label(egui::Color32::YELLOW, "🔄 DHCP DNS Configuration");
            }
            DnsState::None => {
                ui.colored_label(egui::Color32::RED, "❌ No DNS Configuration");
            }
        }
    }

    fn render_provider_selection(&mut self, ui: &mut egui::Ui) {
        let providers = [
            ("Electro", DnsProvider::electro()),
            ("Radar", DnsProvider::radar()),
            ("Shekan", DnsProvider::shekan()),
            ("Bogzar", DnsProvider::bogzar()),
            ("Quad9", DnsProvider::quad9()),
            (
                "Custom",
                DnsProvider::custom(
                    Self::octets_to_ip(&self.custom_primary),
                    Self::octets_to_ip(&self.custom_secondary),
                ),
            ),
        ];

        let current_index = providers
            .iter()
            .position(|(_, provider)| {
                std::mem::discriminant(provider) == std::mem::discriminant(&self.selected_provider)
            })
            .unwrap_or(0);

        egui::ComboBox::from_id_salt("dns_provider")
            .selected_text(providers[current_index].0)
            .show_ui(ui, |ui| {
                for (name, provider) in providers {
                    let was_selected = matches!(
                        (name, &self.selected_provider),
                        ("Custom", DnsProvider::Custom { .. })
                    ) || std::mem::discriminant(&provider)
                        == std::mem::discriminant(&self.selected_provider);

                    if ui.selectable_label(was_selected, name).clicked() {
                        self.selected_provider = provider;
                    }
                }
            });

        if matches!(self.selected_provider, DnsProvider::Custom { .. }) {
            self.selected_provider = DnsProvider::custom(
                Self::octets_to_ip(&self.custom_primary),
                Self::octets_to_ip(&self.custom_secondary),
            );
        }
    }

    fn render_app_state(&self, ui: &mut egui::Ui) {
        match &self.app_state {
            AppState::Idle => {}
            AppState::Processing => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Processing DNS operation...");
                });
            }
            AppState::Success(message) => {
                ui.colored_label(egui::Color32::GREEN, format!("✅ {}", message));
            }
            AppState::Error(message) => {
                ui.colored_label(egui::Color32::RED, format!("❌ {}", message));
            }
            AppState::Warning(message) => {
                ui.colored_label(egui::Color32::YELLOW, format!("⚠️ {}", message));
            }
        }
    }

    fn render_action_buttons(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Set DNS and Clear DNS side-by-side
            ui.horizontal(|ui| {
                if ui
                    .add_sized(
                        Vec2::new(130.0, 50.0),
                        egui::Button::new(
                            egui::RichText::new(format!(
                                "Set {} DNS",
                                self.selected_provider.display_name()
                            ))
                            .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(34, 139, 34))
                        .corner_radius(10),
                    )
                    .clicked()
                {
                    self.handle_operation(DnsOperation::Set(self.selected_provider.clone()));
                }

                ui.add_space(10.0);

                if ui
                    .add_sized(
                        Vec2::new(130.0, 50.0),
                        egui::Button::new(
                            egui::RichText::new("Clear DNS").color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(178, 34, 34))
                        .corner_radius(10),
                    )
                    .clicked()
                {
                    self.handle_operation(DnsOperation::Clear);
                }
            });

            ui.add_space(15.0);

            // Test DNS button as a refresh sticker (icon only, with background)
            ui.horizontal(|ui| {
                ui.add_space(120.0);
                let test_btn = ui
                    .add_sized(
                        Vec2::new(40.0, 40.0),
                        egui::Button::new(egui::RichText::new("🔄").size(28.0)).frame(false),
                    )
                    .on_hover_text("Test DNS")
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                if test_btn.clicked() {
                    self.handle_operation(DnsOperation::Test);
                }
            });
        });
    }

    fn handle_operation(&mut self, operation: DnsOperation) {
        self.app_state = AppState::Processing;

        let adapter = get_active_adapter();
        self.adapter = adapter.clone();

        let (sender, receiver) = mpsc::channel();
        self.operation_sender = Some(sender);
        self.operation_receiver = Some(receiver);

        let adapter_for_thread = adapter;
        let sender_clone = self.operation_sender.clone();

        thread::spawn(move || {
            let result = match operation {
                DnsOperation::Set(provider) => {
                    if let Some(adapter) = &adapter_for_thread {
                        let (primary, secondary) = provider.get_servers();
                        set_dns_with_result(adapter, &primary, &secondary)
                    } else {
                        OperationResult::Error("No Internet Connection Found".to_string())
                    }
                }
                DnsOperation::Clear => {
                    if let Some(adapter) = &adapter_for_thread {
                        clear_dns_with_result(adapter)
                    } else {
                        OperationResult::Error("No Internet Connection Found".to_string())
                    }
                }
                DnsOperation::Test => {
                    if let Some(adapter) = &adapter_for_thread {
                        let dns = get_current_dns(adapter);
                        if dns.is_empty() {
                            OperationResult::Warning("No DNS servers configured".to_string())
                        } else {
                            OperationResult::Success(format!(
                                "DNS test successful: {}",
                                dns.join(", ")
                            ))
                        }
                    } else {
                        OperationResult::Error("No Internet Connection Found".to_string())
                    }
                }
            };

            if let Some(s) = sender_clone {
                let _ = s.send(result);
            }
        });
    }

    fn handle_operation_result(&mut self, result: OperationResult) {
        match result {
            OperationResult::Success(message) => {
                self.app_state = AppState::Success(message);
                if let Some(adapter) = &self.adapter {
                    self.dns = get_current_dns(adapter);
                    self.update_dns_state();
                }
            }
            OperationResult::Error(message) => {
                self.app_state = AppState::Error(message);
            }
            OperationResult::Warning(message) => {
                self.app_state = AppState::Warning(message);
            }
        }
    }

    fn update_dns_state(&mut self) {
        if self.dns.is_empty() {
            self.dns_state = DnsState::None;
        } else if self.dns.len() == 1 && self.dns[0].contains("dhcp") {
            self.dns_state = DnsState::Dhcp;
        } else {
            self.dns_state = DnsState::Static(self.dns.clone());
        }
    }

    /// Convert octet array to IP address string.
    fn octets_to_ip(octets: &[String; 4]) -> String {
        octets.join(".")
    }

    fn render_secondary_viewport(&mut self, ctx: &egui::Context) {
        if !self.show_second_window {
            return;
        }
        // prepare values to move into the closure (avoid capturing &mut self)
        let ping_value = self.ping_value;
        let ping_text = format!("{} ms", ping_value);
        // choose color by threshold: <100 green, 100-199 yellow, >=200 red, 0 = light gray (error/no response)
        let ping_color = if ping_value == 0.0 {
            egui::Color32::LIGHT_GRAY
        } else if ping_value < 100.0 {
            egui::Color32::GREEN
        } else if ping_value < 200.0 {
            egui::Color32::YELLOW
        } else {
            egui::Color32::RED
        };

        let keep_open = std::cell::Cell::new(true);
        let window_size = egui::vec2(200.0, 180.0);
        let screen_center = ctx.input(|i| {
            let info = i.viewport();
            info.outer_rect
                .or(info.inner_rect)
                .map(|rect| rect.center())
                .unwrap_or_else(|| egui::pos2(0.0, 0.0))
        });
        let position = screen_center - window_size / 2.0;
        let viewport_id = egui::ViewportId::from_hash_of("ping");
        ctx.show_viewport_immediate(
            viewport_id,
            egui::ViewportBuilder::default()
                .with_title("Ping")
                .with_inner_size(window_size)
                .with_position(position)
                .with_resizable(true)
                .with_decorations(true),
            {
                let keep_open = &keep_open;
                move |ctx, _class| {
                    if ctx.input(|i| i.viewport().close_requested()) {
                        keep_open.set(false);
                    }

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.heading(" Ping Monitor");
                            ui.add_space(20.0);
                            ui.label(
                                egui::RichText::new(ping_text.clone())
                                    .size(28.0)
                                    .color(ping_color),
                            );
                        });
                    });
                }
            },
        );

        self.show_second_window = keep_open.get();
        if !self.show_second_window {
            let _ = self.ping_sender.take();
            self.ping_receiver = None;
            self.ping_value = 0.0;
        }
    }
}

fn custom_window_frame(ctx: &egui::Context, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    use egui::{CentralPanel, UiBuilder};

    let panel_frame = egui::Frame::new()
        .fill(ctx.style().visuals.window_fill())
        .corner_radius(10)
        .stroke(ctx.style().visuals.widgets.noninteractive.fg_stroke)
        .outer_margin(1);

    CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
        let app_rect = ui.max_rect();

        let title_bar_height = 40.0;
        let title_bar_rect = {
            let mut rect = app_rect;
            rect.max.y = rect.min.y + title_bar_height;
            rect
        };
        title_bar_ui(ui, title_bar_rect, title);

        let content_rect = {
            let mut rect = app_rect;
            rect.min.y = title_bar_rect.max.y;
            rect
        }
        .shrink(4.0);

        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect));
        add_contents(&mut content_ui);
    });
}

fn title_bar_ui(ui: &mut egui::Ui, title_bar_rect: eframe::epaint::Rect, title: &str) {
    use egui::{vec2, Align2, FontId, Id, PointerButton, Sense, UiBuilder, ViewportCommand};

    let painter = ui.painter();

    let title_bar_response = ui.interact(
        title_bar_rect,
        Id::new("title_bar"),
        Sense::click_and_drag(),
    );

    painter.text(
        title_bar_rect.center(),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(20.0),
        ui.style().visuals.text_color(),
    );

    painter.line_segment(
        [
            title_bar_rect.left_bottom() + vec2(1.0, 0.0),
            title_bar_rect.right_bottom() + vec2(-1.0, 0.0),
        ],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );

    // Left-side (top-left) controls: ping button
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(title_bar_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.visuals_mut().button_frame = false;
            ui.add_space(6.0);

            let button_height = 20.0;
            let ping_btn = ui
                .add(egui::Button::new(
                    egui::RichText::new("📶").size(button_height),
                ))
                .on_hover_text("Ping Monitor")
                .on_hover_cursor(egui::CursorIcon::PointingHand);

            if ping_btn.clicked() {
                // Signal the main update loop to start pinging and open the secondary window
                PING_REQUEST.store(true, Ordering::SeqCst);
            }

            // keep remaining left-side space empty
            ui.add_space(4.0);
        },
    );

    if title_bar_response.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    ui.scope_builder(
        UiBuilder::new()
            .max_rect(title_bar_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.visuals_mut().button_frame = false;
            ui.add_space(8.0);
            close_button(ui);
        },
    );
}

/// Show a close button for the native window.
fn close_button(ui: &mut egui::Ui) {
    use egui::{Button, RichText, ViewportCommand};

    let button_height = 20.0;

    let close_resp = ui
        .add(Button::new(RichText::new("❌").size(button_height)))
        .on_hover_text("Close the window")
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    if close_resp.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }
}

fn get_ping() -> f64 {
    let target_ip = "8.8.8.8".parse::<std::net::IpAddr>().expect("invalid IP");

    let mut p = ping::new(target_ip);
    p.timeout(std::time::Duration::from_secs(2)).ttl(128);

    let start = Instant::now();

    match p.send() {
        Ok(_) => start.elapsed().as_millis() as f64,
        Err(_) => 0.0, // return 0 on error
    }
}
