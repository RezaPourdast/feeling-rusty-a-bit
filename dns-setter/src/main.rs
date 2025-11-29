//! DNS Setter - Windows GUI for managing DNS settings

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Vec2};
use regex::Regex;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

const CREATE_NO_WINDOW: u32 = 0x08000000; // Hide console window

/// Represents different DNS providers with their server configurations
#[derive(Debug, Clone, PartialEq)]
enum DnsProvider {
    Electro { primary: String, secondary: String },
    Radar { primary: String, secondary: String },
    Shekan { primary: String, secondary: String },
    Bogzar { primary: String, secondary: String },
    Quad9 { primary: String, secondary: String },
    Custom { primary: String, secondary: String },
}

impl DnsProvider {
    /// Create Electro DNS provider
    fn electro() -> Self {
        Self::Electro {
            primary: "78.157.42.100".to_string(),
            secondary: "78.157.42.101".to_string(),
        }
    }
    /// Create Radar DNS provider
    fn radar() -> Self {
        Self::Radar {
            primary: "10.202.10.10".to_string(),
            secondary: "10.202.10.11".to_string(),
        }
    }
    /// Create Shekan DNS provider
    fn shekan() -> Self {
        Self::Shekan {
            primary: "178.22.122.100".to_string(),
            secondary: "185.51.200.2".to_string(),
        }
    }
    /// Create Bogzar DNS provider
    fn bogzar() -> Self {
        Self::Bogzar {
            primary: "185.55.226.26".to_string(),
            secondary: "185.55.225.25".to_string(),
        }
    }
    /// Create Quad9 DNS provider
    fn quad9() -> Self {
        Self::Quad9 {
            primary: "9.9.9.9".to_string(),
            secondary: "149.112.112.112".to_string(),
        }
    }
    /// Create custom DNS provider
    fn custom(primary: String, secondary: String) -> Self {
        Self::Custom { primary, secondary }
    }

    /// Get DNS servers as tuple
    fn get_servers(&self) -> (String, String) {
        match self {
            DnsProvider::Electro { primary, secondary }
            | DnsProvider::Radar { primary, secondary }
            | DnsProvider::Shekan { primary, secondary }
            | DnsProvider::Bogzar { primary, secondary }
            | DnsProvider::Quad9 { primary, secondary }
            | DnsProvider::Custom { primary, secondary } => (primary.clone(), secondary.clone()),
        }
    }

    /// Get display name for UI
    fn display_name(&self) -> &'static str {
        match self {
            DnsProvider::Electro { .. } => "Electro",
            DnsProvider::Radar { .. } => "Radar",
            DnsProvider::Shekan { .. } => "Shekan",
            DnsProvider::Bogzar { .. } => "Bogzar",
            DnsProvider::Quad9 { .. } => "Quad9",
            DnsProvider::Custom { .. } => "Custom",
        }
    }

    /// Get description for UI
    fn description(&self) -> &'static str {
        match self {
            DnsProvider::Electro { .. } => "Fast gaming DNS",
            DnsProvider::Radar { .. } => "Fast gaming DNS",
            DnsProvider::Shekan { .. } => "Fast gaming DNS",
            DnsProvider::Bogzar { .. } => "Fast gaming DNS",
            DnsProvider::Quad9 { .. } => "Security-focused",
            DnsProvider::Custom { .. } => "User-defined servers",
        }
    }
}

/// Represents different DNS operations
#[derive(Debug, Clone, PartialEq)]
enum DnsOperation {
    Set(DnsProvider),
    Clear,
    Refresh,
    Test,
}

/// Represents the result of a DNS operation
#[derive(Debug, Clone, PartialEq)]
enum OperationResult {
    Success(String),
    Error(String),
    Warning(String),
    Info(String),
}

/// Represents the current state of the application
#[derive(Debug, Clone, PartialEq)]
enum AppState {
    Idle,
    Processing,
    Success(String),
    Error(String),
    Warning(String),
}

/// Represents network adapter states
#[derive(Debug, Clone, PartialEq)]
enum AdapterState {
    Connected,
    Disconnected,
    Unknown,
}

/// Represents DNS configuration states
#[derive(Debug, Clone, PartialEq)]
enum DnsState {
    Static(Vec<String>),
    Dhcp,
    None,
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Idle
    }
}

impl Default for DnsProvider {
    fn default() -> Self {
        DnsProvider::electro()
    }
}

impl Default for DnsState {
    fn default() -> Self {
        DnsState::None
    }
}

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([400.0, 600.0])
            .with_min_inner_size([400.0, 600.0])
            .with_transparent(true),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "DNS SETTER",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp {
                adapter: None,
                dns: Vec::new(),
                app_state: AppState::default(),
                selected_provider: DnsProvider::default(),
                dns_state: DnsState::None,
                custom_primary: [String::new(), String::new(), String::new(), String::new()],
                custom_secondary: [String::new(), String::new(), String::new(), String::new()],
                operation_sender: None,
                operation_receiver: None,
            }))
        }),
    )
}

#[derive(Default)]
struct MyApp {
    adapter: Option<String>,
    dns: Vec<String>,
    app_state: AppState,
    selected_provider: DnsProvider,
    dns_state: DnsState,
    custom_primary: [String; 4],
    custom_secondary: [String; 4],
    operation_sender: Option<mpsc::Sender<OperationResult>>,
    operation_receiver: Option<mpsc::Receiver<OperationResult>>,
}

impl eframe::App for MyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for operation results from background thread
        if let Some(receiver) = &self.operation_receiver {
            if let Ok(result) = receiver.try_recv() {
                self.handle_operation_result(result);
                self.operation_receiver = None;
                self.operation_sender = None;
                ctx.request_repaint();
            } else if matches!(self.app_state, AppState::Processing) {
                // Keep requesting repaint to show spinner animation while processing
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

                // Show custom DNS input fields when Custom is selected
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
                    ui.add_space(45.0);
                    self.render_action_buttons(ui);
                });
            });
        });
    }
}

impl MyApp {
    /// Render an IP address input with 4 separate octet fields
    fn render_ip_input(ui: &mut egui::Ui, octets: &mut [String; 4], label: &str) {
        ui.horizontal(|ui| {
            ui.label(format!("{}: ", label));

            for (i, octet) in octets.iter_mut().enumerate() {
                // Limit input to 3 characters and only allow digits
                let response = ui.add_sized(
                    Vec2::new(40.0, 20.0),
                    egui::TextEdit::singleline(octet)
                        .desired_width(40.0)
                        .char_limit(3),
                );

                // Filter out non-digit characters
                if response.changed() {
                    *octet = octet.chars().filter(|c| c.is_ascii_digit()).collect();

                    // Auto-advance to next field if 3 digits entered
                    if octet.len() == 3 && i < 3 {
                        // Focus next field (egui will handle this automatically on next frame)
                    }
                }

                // Add dot separator (except after last octet)
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
                ui.label(format!(
                    "Primary: {}",
                    servers.get(0).unwrap_or(&"None".to_string())
                ));
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
        // Provider options for combobox
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

        // Find current selection index
        let current_index = providers
            .iter()
            .position(|(_, provider)| {
                std::mem::discriminant(provider) == std::mem::discriminant(&self.selected_provider)
            })
            .unwrap_or(0);

        // Create combobox
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

        // Update custom provider if Custom is selected and inputs changed
        if matches!(self.selected_provider, DnsProvider::Custom { .. }) {
            self.selected_provider = DnsProvider::custom(
                Self::octets_to_ip(&self.custom_primary),
                Self::octets_to_ip(&self.custom_secondary),
            );
        }
    }

    /// Render the application state with appropriate colors
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
        ui.horizontal(|ui| {
            // Left column
            ui.vertical(|ui| {
                // Button 1: Set DNS
                if ui
                    .add_sized(
                        Vec2::new(140.0, 50.0),
                        egui::Button::new(format!(
                            "Set {} DNS",
                            self.selected_provider.display_name()
                        ))
                        .corner_radius(10),
                    )
                    .clicked()
                {
                    self.handle_operation(DnsOperation::Set(self.selected_provider.clone()));
                }

                ui.add_space(10.0);

                // Button 2: Clear DNS
                if ui
                    .add_sized(
                        Vec2::new(140.0, 50.0),
                        egui::Button::new("Clear DNS").corner_radius(10),
                    )
                    .clicked()
                {
                    self.handle_operation(DnsOperation::Clear);
                }
            });

            ui.add_space(5.0);

            // Right column
            ui.vertical(|ui| {
                // Button 3: Refresh
                if ui
                    .add_sized(
                        Vec2::new(140.0, 50.0),
                        egui::Button::new("Refresh").corner_radius(10),
                    )
                    .clicked()
                {
                    self.handle_operation(DnsOperation::Refresh);
                }

                ui.add_space(10.0);

                // Button 4: Test DNS
                if ui
                    .add_sized(
                        Vec2::new(140.0, 50.0),
                        egui::Button::new("Test DNS").corner_radius(10),
                    )
                    .clicked()
                {
                    self.handle_operation(DnsOperation::Test);
                }
            });
        });
    }

    fn handle_operation(&mut self, operation: DnsOperation) {
        self.app_state = AppState::Processing;

        // Get adapter name before spawning thread
        let adapter = get_active_adapter();
        self.adapter = adapter.clone();

        // Create channel for communication between threads
        let (sender, receiver) = mpsc::channel();
        self.operation_sender = Some(sender);
        self.operation_receiver = Some(receiver);

        // Clone necessary data for the thread
        let adapter_for_thread = adapter;

        // Spawn background thread for DNS operations
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
                DnsOperation::Refresh => {
                    if let Some(adapter) = &adapter_for_thread {
                        let dns = get_current_dns(adapter);
                        if dns.is_empty() {
                            OperationResult::Warning("No DNS servers found".to_string())
                        } else {
                            OperationResult::Success(format!(
                                "DNS status refreshed: {}",
                                dns.join(", ")
                            ))
                        }
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
                // Refresh DNS state after successful operation
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
            OperationResult::Info(message) => {
                self.app_state = AppState::Success(message);
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

    /// Convert octet array to IP address string
    fn octets_to_ip(octets: &[String; 4]) -> String {
        octets.join(".")
    }
}

fn custom_window_frame(ctx: &egui::Context, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    use egui::{CentralPanel, UiBuilder};

    let panel_frame = egui::Frame::new()
        .fill(ctx.style().visuals.window_fill())
        .corner_radius(10)
        .stroke(ctx.style().visuals.widgets.noninteractive.fg_stroke)
        .outer_margin(1); // keep stroke within bounds

    CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
        let app_rect = ui.max_rect();

        let title_bar_height = 32.0;
        let title_bar_rect = {
            let mut rect = app_rect;
            rect.max.y = rect.min.y + title_bar_height;
            rect
        };
        title_bar_ui(ui, title_bar_rect, title);

        // Contents area:
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
    use egui::{Align2, FontId, Id, PointerButton, Sense, UiBuilder, ViewportCommand, vec2};

    let painter = ui.painter();

    let title_bar_response = ui.interact(
        title_bar_rect,
        Id::new("title_bar"),
        Sense::click_and_drag(),
    );

    // Title text:
    painter.text(
        title_bar_rect.center(),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(20.0),
        ui.style().visuals.text_color(),
    );

    // Underline:
    painter.line_segment(
        [
            title_bar_rect.left_bottom() + vec2(1.0, 0.0),
            title_bar_rect.right_bottom() + vec2(-1.0, 0.0),
        ],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );

    // Drag to move window:
    if title_bar_response.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    // Right-side window controls:
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

/// Show some close button for the native window.
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

fn run_netsh(args: &[&str]) -> std::process::Output {
    Command::new("netsh")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to run netsh")
        .wait_with_output()
        .expect("Failed to wait for netsh")
}

fn get_active_adapter() -> Option<String> {
    let output = run_netsh(&["interface", "show", "interface"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.contains("Connected") && line.contains("Dedicated") {
            // Extract adapter name (last word in the line)
            let parts: Vec<&str> = line.split_whitespace().collect();
            return parts.last().map(|s| s.to_string());
        }
    }
    None
}

fn get_current_dns(adapter: &str) -> Vec<String> {
    let output = run_netsh(&[
        "interface",
        "ip",
        "show",
        "dns",
        &format!("name={}", adapter),
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    let re = Regex::new(r"\b\d{1,3}(?:\.\d{1,3}){3}\b").unwrap(); // Match IPv4 addresses
    re.find_iter(&stdout)
        .map(|m| m.as_str().to_string())
        .collect()
}

fn clear_dns(interface: &str) {
    let output = run_netsh(&[
        "interface",
        "ipv4",
        "set",
        "dns",
        &format!("name={}", interface),
        "source=dhcp",
    ]);
    if output.status.success() {
        println!("✅ DNS reset to DHCP successfully for '{}'.", interface);
    } else {
        eprintln!(
            "❌ Error resetting DNS for '{}':\n{}",
            interface,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn set_dns(interface: &str, primary: &str, secondary: &str) {
    let output1 = run_netsh(&[
        "interface",
        "ipv4",
        "set",
        "dns",
        &format!("name={}", interface),
        "static",
        primary,
    ]);
    if output1.status.success() {
        println!(
            "Primary DNS {} set successfully for '{}'",
            primary, interface
        );
    } else {
        eprintln!(
            "Error setting primary DNS {}: {}",
            primary,
            String::from_utf8_lossy(&output1.stderr)
        );
    }

    let output2 = run_netsh(&[
        "interface",
        "ipv4",
        "add",
        "dns",
        &format!("name={}", interface),
        secondary,
        "index=2",
    ]);
    if output2.status.success() {
        println!(
            "Secondary DNS {} set successfully for '{}'",
            secondary, interface
        );
    } else {
        eprintln!(
            "Error setting secondary DNS {}: {}",
            secondary,
            String::from_utf8_lossy(&output2.stderr)
        );
    }
}

fn set_dns_with_result(interface: &str, primary: &str, secondary: &str) -> OperationResult {
    let output1 = run_netsh(&[
        "interface",
        "ipv4",
        "set",
        "dns",
        &format!("name={}", interface),
        "static",
        primary,
    ]);

    if !output1.status.success() {
        return OperationResult::Error(format!(
            "Error setting primary DNS {}: {}",
            primary,
            String::from_utf8_lossy(&output1.stderr)
        ));
    }

    let output2 = run_netsh(&[
        "interface",
        "ipv4",
        "add",
        "dns",
        &format!("name={}", interface),
        secondary,
        "index=2",
    ]);

    if !output2.status.success() {
        return OperationResult::Error(format!(
            "Error setting secondary DNS {}: {}",
            secondary,
            String::from_utf8_lossy(&output2.stderr)
        ));
    }

    OperationResult::Success(format!(
        "DNS servers {} and {} set successfully for '{}'",
        primary, secondary, interface
    ))
}

fn clear_dns_with_result(interface: &str) -> OperationResult {
    let output = run_netsh(&[
        "interface",
        "ipv4",
        "set",
        "dns",
        &format!("name={}", interface),
        "source=dhcp",
    ]);
    if output.status.success() {
        OperationResult::Success(format!(
            "DNS reset to DHCP successfully for '{}'",
            interface
        ))
    } else {
        OperationResult::Error(format!(
            "Error resetting DNS for '{}': {}",
            interface,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
