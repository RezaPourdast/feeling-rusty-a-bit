//! Application state, egui integration, and UI rendering.

use ping;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{self, ColorImage, TextureHandle, Vec2};
use image;

use crate::domain::{AppState, DnsOperation, DnsProvider, DnsState, OperationResult};
use crate::system::{
    clear_dns_with_result, get_active_adapter, get_current_dns, set_dns_with_result,
};

// ============================================================================
// UI CONSTANTS & THEME
// ============================================================================

/// UI spacing constants
mod ui_constants {
    pub const SPACING_SMALL: f32 = 10.0;
    pub const SPACING_MEDIUM: f32 = 20.0;
    pub const _SPACING_LARGE: f32 = 30.0;
    pub const _SPACING_XLARGE: f32 = 40.0;

    pub const BUTTON_WIDTH: f32 = 200.0;
    pub const BUTTON_HEIGHT: f32 = 40.0;
    pub const BUTTON_SPACING: f32 = 3.0;

    pub const TITLE_BAR_HEIGHT: f32 = 40.0;
    pub const _WINDOW_PADDING: f32 = 4.0; // Reserved for future use
}

/// UI color constants
mod ui_colors {
    use eframe::egui::Color32;

    pub const BUTTON_SUCCESS: Color32 = Color32::from_rgb(60, 140, 64); // Darker #4CAF50
    pub const BUTTON_DANGER: Color32 = Color32::from_rgb(183, 46, 42); // Darker #E53935
    pub const BUTTON_TEXT: Color32 = Color32::WHITE;

    pub const STATUS_STATIC: Color32 = Color32::GREEN;
    pub const STATUS_DHCP: Color32 = Color32::YELLOW;
    pub const STATUS_NONE: Color32 = Color32::RED;

    pub const SUCCESS: Color32 = Color32::GREEN;
    pub const ERROR: Color32 = Color32::RED;
    pub const WARNING: Color32 = Color32::YELLOW;
}

/// Configure UI theme and styling
fn configure_theme(ctx: &egui::Context) {
    use ui_constants::*;

    let mut style = (*ctx.style()).clone();

    // Configure spacing
    style.spacing.item_spacing = egui::vec2(SPACING_SMALL, SPACING_SMALL);

    // Configure visuals (optional - customize as needed)
    // style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 45, 48);
    // style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 60, 65);
    // style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(70, 70, 75);

    ctx.set_style(style);
}

// Track if theme has been configured
static THEME_CONFIGURED: AtomicBool = AtomicBool::new(false);

/// Main application container used by eframe.
#[derive(Default)]
pub struct MyApp {
    adapter: Option<String>,
    dns: Vec<String>,
    app_state: AppState,
    selected_provider: DnsProvider,
    dns_state: DnsState,
    custom_primary: String,
    custom_secondary: String,
    operation_sender: Option<mpsc::Sender<OperationResult>>,
    operation_receiver: Option<mpsc::Receiver<OperationResult>>,
    show_second_window: bool,
    ping_value: f64,
    ping_history: VecDeque<f64>,
    ping_sender: Option<mpsc::Sender<f64>>,
    ping_receiver: Option<mpsc::Receiver<f64>>,
    show_clear_confirmation: bool,
    show_custom_dns_window: bool,
    background_texture: Option<TextureHandle>,
    ping_background_texture: Option<TextureHandle>,
    custom_dns_background_texture: Option<TextureHandle>,
    social_logos: std::collections::HashMap<String, TextureHandle>,
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
            ping_history: VecDeque::with_capacity(15), // Keep only last 15 data points
            ping_sender: None,
            ping_receiver: None,
            background_texture: None,
            ping_background_texture: None,
            custom_dns_background_texture: None,
            social_logos: std::collections::HashMap::new(),
            ..Default::default()
        };

        app
    }

    fn load_background_image(&mut self, ctx: &egui::Context) {
        // Try to load main background image from asset folder
        let image_path = if let Ok(dir) = std::env::current_dir() {
            dir.join("asset").join("main-background.png")
        } else {
            std::path::PathBuf::from("asset/main-background.png")
        };

        // Try PNG first, then JPG, then WEBP
        let paths = vec![
            image_path.clone(),
            image_path.with_extension("jpg"),
            image_path.with_extension("jpeg"),
            image_path.with_extension("webp"),
        ];

        for path in paths {
            if path.exists() {
                // Load image using image crate
                if let Ok(img) = image::open(&path) {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.as_flat_samples();
                    let color_image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    let texture =
                        ctx.load_texture("background", color_image, egui::TextureOptions::LINEAR);
                    self.background_texture = Some(texture);
                    break;
                }
            }
        }
    }

    fn load_ping_background_image(&mut self, ctx: &egui::Context) {
        // Try to load ping background image from asset folder
        let image_path = if let Ok(dir) = std::env::current_dir() {
            dir.join("asset").join("ping-background.png")
        } else {
            std::path::PathBuf::from("asset/ping-background.png")
        };

        // Try PNG first, then JPG, then WEBP
        let paths = vec![
            image_path.clone(),
            image_path.with_extension("jpg"),
            image_path.with_extension("jpeg"),
            image_path.with_extension("webp"),
        ];

        for path in paths {
            if path.exists() {
                // Load image using image crate
                if let Ok(img) = image::open(&path) {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.as_flat_samples();
                    let color_image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    let texture = ctx.load_texture(
                        "ping_background",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.ping_background_texture = Some(texture);
                    break;
                }
            }
        }
    }

    fn load_custom_dns_background_image(&mut self, ctx: &egui::Context) {
        // Try to load custom DNS background image from asset folder
        let image_path = if let Ok(dir) = std::env::current_dir() {
            dir.join("asset").join("custom-dns-bg.png")
        } else {
            std::path::PathBuf::from("asset/custom-dns-bg.png")
        };

        // Try PNG first, then JPG, then WEBP
        let paths = vec![
            image_path.clone(),
            image_path.with_extension("jpg"),
            image_path.with_extension("jpeg"),
            image_path.with_extension("webp"),
        ];

        for path in paths {
            if path.exists() {
                // Load image using image crate
                if let Ok(img) = image::open(&path) {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.as_flat_samples();
                    let color_image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    let texture = ctx.load_texture(
                        "custom_dns_background",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.custom_dns_background_texture = Some(texture);
                    break;
                }
            }
        }
    }

    fn load_social_logos(&mut self, ctx: &egui::Context) {
        // Load the three logos: cup-of-drink, email, github
        let logo_files = vec![
            ("cup-of-drink", "cup-of-drink.png"),
            ("email", "email.png"),
            ("github", "github.png"),
        ];

        for (name, filename) in logo_files {
            let image_path = if let Ok(dir) = std::env::current_dir() {
                dir.join("asset").join(filename)
            } else {
                std::path::PathBuf::from(format!("asset/{}", filename))
            };

            // Try multiple formats
            let paths = vec![
                image_path.clone(),
                image_path.with_extension("jpg"),
                image_path.with_extension("jpeg"),
                image_path.with_extension("webp"),
            ];

            for path in paths {
                if path.exists() {
                    if let Ok(img) = image::open(&path) {
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.as_flat_samples();
                        let color_image =
                            ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                        let texture = ctx.load_texture(
                            format!("logo_{}", name),
                            color_image,
                            egui::TextureOptions::LINEAR,
                        );
                        self.social_logos.insert(name.to_string(), texture);
                        break;
                    }
                }
            }
        }
    }

    fn render_footer(&mut self, ui: &mut egui::Ui) {
        // Just the clickable logo links (no footer bar) - horizontal layout
        let icon_size = 28.0;
        let icon_spacing = 15.0;
        let light_gray = egui::Color32::from_rgb(180, 180, 180); // Light gray color

        ui.vertical(|ui| {
            ui.add_space(40.0);
            ui.horizontal(|ui| {
                ui.add_space(52.5);
                // Define logos with their URLs
                let logos = vec![
                    ("cup-of-drink", "https://www.coffeete.ir/rezapourdast"),
                    ("email", "mailto:s.rezapourdast@gmail.com"),
                    ("github", "https://github.com/RezaPourdast"),
                ];

                for (logo_name, url) in logos {
                    if let Some(texture) = self.social_logos.get(logo_name) {
                        // Create clickable area for the logo
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(icon_size, icon_size),
                            egui::Sense::click(),
                        );

                        // Draw the image with light gray tint
                        let painter = ui.painter();
                        painter.image(
                            texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            light_gray,
                        );

                        // Check for click first
                        if response.clicked() {
                            // Open URL
                            let _ = open::that(url);
                        }

                        // Add hover effect
                        if response.hovered() {
                            painter.rect_filled(
                                rect,
                                0.0,
                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                            );
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        ui.add_space(icon_spacing);
                    }
                }
            });
        });
    }
}

impl eframe::App for MyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Configure theme once on first update
        if !THEME_CONFIGURED.swap(true, Ordering::SeqCst) {
            configure_theme(ctx);
        }

        // Load background image on first update
        if self.background_texture.is_none() {
            self.load_background_image(ctx);
        }

        // Load ping background image on first update
        if self.ping_background_texture.is_none() {
            self.load_ping_background_image(ctx);
        }

        // Load custom DNS background image on first update
        if self.custom_dns_background_texture.is_none() {
            self.load_custom_dns_background_image(ctx);
        }

        // Load social logos on first update
        if self.social_logos.is_empty() {
            self.load_social_logos(ctx);
        }

        // Store background texture in context for custom_window_frame to access
        if let Some(ref texture) = self.background_texture {
            ctx.data_mut(|d| {
                d.insert_temp(egui::Id::new("background_texture"), Some(texture.clone()));
            });
        }

        // Store ping background texture in context for ping window to access
        if let Some(ref texture) = self.ping_background_texture {
            ctx.data_mut(|d| {
                d.insert_temp(
                    egui::Id::new("ping_background_texture"),
                    Some(texture.clone()),
                );
            });
        }

        // Store custom DNS background texture in context for custom DNS window to access
        if let Some(ref texture) = self.custom_dns_background_texture {
            ctx.data_mut(|d| {
                d.insert_temp(
                    egui::Id::new("custom_dns_background_texture"),
                    Some(texture.clone()),
                );
            });
        }

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
                // Add to history, keeping only last 5 values
                // Keep only last 15 data points
                if self.ping_history.len() >= 15 {
                    self.ping_history.pop_front();
                }
                self.ping_history.push_back(ping);
                ctx.request_repaint();
            }
        }

        custom_window_frame(ctx, "", |ui| {
            use ui_constants::*;

            // Status Section - wrapped in a card with fixed width, rounded corners, and transparent blur effect
            ui.horizontal(|ui| {
                ui.set_max_width(230.0);
                ui.set_max_height(165.0);
                // Custom frame with transparent background and rounded corners
                // Using semi-transparent dark color for blur/frosted glass effect
                let frame = egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(60, 60, 65, 45)) // Lighter gray with higher opacity for blurry effect
                    .corner_radius(12.0); // Increased corner radius
                frame.show(ui, |ui| {
                    ui.set_width(225.0);
                    ui.set_height(165.0);
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.add_space(12.0);
                        self.render_status_section(ui);
                        self.render_app_state(ui);
                    });
                });
            });

            // DNS List Section - same transparent frame style without max height
            ui.horizontal(|ui| {
                ui.set_max_width(230.0);
                // Custom frame with transparent background and rounded corners (same as status section)
                let frame = egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(60, 60, 65, 45)) // Lighter gray with higher opacity for blurry effect
                    .corner_radius(12.0); // Same rounded corners
                frame.show(ui, |ui| {
                    ui.set_width(225.0);
                    // No max height constraint - let it grow with content
                    ui.vertical(|ui| {
                        ui.add_space(12.0);
                        // Label above dropdown
                        ui.horizontal(|ui| {
                            ui.add_space(13.0);
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("DNS List")
                                        .color(egui::Color32::WHITE)
                                        .size(18.0), // Larger font size
                                );
                                self.render_provider_selection(ui);
                            });
                        });
                        ui.add_space(BUTTON_SPACING);
                        self.render_action_buttons(ui);
                    });
                });
            });

            // Footer with clickable logo links
            self.render_footer(ui);
        });

        // If the title-bar ping button was clicked, start the ping thread / open the window.
        if PING_REQUEST.swap(false, Ordering::SeqCst) {
            if self.ping_sender.is_none() {
                let (tx, rx) = mpsc::channel::<f64>();
                self.ping_sender = Some(tx.clone());
                self.ping_receiver = Some(rx);

                thread::spawn(move || {
                    loop {
                        let value = get_ping();
                        if tx.send(value).is_err() {
                            break;
                        }
                        thread::sleep(Duration::from_secs(1));
                    }
                });
            }
            self.show_second_window = true;
        }

        self.render_secondary_viewport(ctx);
        self.render_custom_dns_window(ctx);

        // Show confirmation dialog for Clear DNS
        if self.show_clear_confirmation {
            use ui_colors::{BUTTON_SUCCESS, BUTTON_TEXT};

            egui::Window::new("Confirm Clear DNS")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(
                            "Are you sure you want to clear the DNS configuration?",
                        )
                        .color(egui::Color32::WHITE),
                    );
                    ui.label(
                        egui::RichText::new("This will reset DNS to DHCP/automatic.")
                            .color(egui::Color32::WHITE),
                    );
                    ui.add_space(10.0);

                    // Buttons at bottom right with margin
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        // Add margin from right
                        ui.add_space(10.0);

                        // Cancel button (transparent gray) - right side
                        if ui
                            .add_sized(
                                Vec2::new(80.0, 30.0),
                                egui::Button::new(egui::RichText::new("Cancel").color(BUTTON_TEXT))
                                    .fill(egui::Color32::from_rgba_unmultiplied(100, 100, 100, 100)) // Transparent gray
                                    .corner_radius(6),
                            )
                            .clicked()
                        {
                            self.show_clear_confirmation = false;
                        }

                        ui.add_space(3.0); // Closer spacing

                        // Clear DNS button (green) - left side
                        if ui
                            .add_sized(
                                Vec2::new(80.0, 30.0),
                                egui::Button::new(
                                    egui::RichText::new("Clear DNS").color(BUTTON_TEXT),
                                )
                                .fill(BUTTON_SUCCESS)
                                .corner_radius(6),
                            )
                            .clicked()
                        {
                            self.show_clear_confirmation = false;
                            self.handle_operation(DnsOperation::Clear);
                        }
                    });
                });
        }

        ctx.request_repaint_after(Duration::from_millis(1000));
    }
}

impl MyApp {
    fn render_ip_input(ui: &mut egui::Ui, ip: &mut String, label: &str) -> bool {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{}: ", label)).color(egui::Color32::WHITE));

            let field_id = egui::Id::new(label);
            // Check validation before creating text_edit to avoid borrow issues
            let ip_clone = ip.clone();
            let is_valid = ip_clone.is_empty() || Self::is_valid_ip(&ip_clone);

            let mut text_edit = egui::TextEdit::singleline(ip)
                .desired_width(200.0)
                .id(field_id)
                .text_color(egui::Color32::WHITE); // Default white text

            if !ip_clone.is_empty() && !is_valid {
                text_edit = text_edit.text_color(egui::Color32::RED);
            }

            ui.add_sized(Vec2::new(200.0, 20.0), text_edit);
        });

        // Validate after rendering
        ip.is_empty() || Self::is_valid_ip(ip)
    }

    fn is_valid_ip(ip: &str) -> bool {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        for part in parts {
            // parse::<u8>() already ensures the value is 0-255
            if part.parse::<u8>().is_err() {
                return false;
            }
        }
        true
    }

    fn render_status_section(&mut self, ui: &mut egui::Ui) {
        use ui_colors::{STATUS_DHCP, STATUS_NONE, STATUS_STATIC};

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Current Status")
                        .color(egui::Color32::WHITE)
                        .size(18.0),
                );
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    let test_btn = ui
                        .add_sized(
                            Vec2::new(22.0, 22.0), // Smaller button size
                            egui::Button::new(egui::RichText::new("🔄").size(16.0)).frame(false),
                        )
                        .on_hover_text("Test DNS")
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if test_btn.clicked() {
                        self.handle_operation(DnsOperation::Test);
                    }
                });
            });
        });

        match &self.dns_state {
            DnsState::Static(servers) => {
                ui.colored_label(STATUS_STATIC, "Static DNS Configuration 🔒");
                let fallback = String::from("None");
                let primary = servers.first().unwrap_or(&fallback);
                ui.label(
                    egui::RichText::new(format!("Primary: {}", primary))
                        .color(egui::Color32::WHITE),
                );
                if servers.len() > 1 {
                    let secondary = servers.get(1).unwrap_or(&fallback);
                    ui.label(
                        egui::RichText::new(format!("Secondary: {}", secondary))
                            .color(egui::Color32::WHITE),
                    );
                }
            }
            DnsState::Dhcp => {
                ui.colored_label(STATUS_DHCP, "🔄 DHCP DNS Configuration");
            }
            DnsState::None => {
                ui.colored_label(STATUS_NONE, "❌ No DNS Configuration");
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
                DnsProvider::custom(self.custom_primary.clone(), self.custom_secondary.clone()),
            ),
        ];

        let current_index = providers
            .iter()
            .position(|(_, provider)| {
                std::mem::discriminant(provider) == std::mem::discriminant(&self.selected_provider)
            })
            .unwrap_or(0);

        // Center the combobox with button size, transparent background, and rounded corners
        use ui_constants::{BUTTON_HEIGHT, BUTTON_WIDTH};
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            // Store original styles
            let original_padding = ui.style().spacing.button_padding;
            let original_bg_fill = ui.style().visuals.widgets.inactive.bg_fill;
            let original_corner_radius = ui.style().visuals.widgets.inactive.corner_radius;

            // Calculate padding to achieve button height
            let text_size = ui.style().text_styles[&egui::TextStyle::Body].size;
            let vertical_padding = ((BUTTON_HEIGHT / 2.0 + 5.0) - text_size) / 2.0;
            ui.style_mut().spacing.button_padding = egui::vec2(8.0, vertical_padding.max(0.0));

            // Make combobox semi-transparent with rounded corners like buttons
            // Set all widget states to have slight background opacity with darker gray
            let bg_opacity = 45; // Semi-transparent background
            let dark_gray = 255; // Dark gray color (60, 60, 60)
            ui.style_mut().visuals.widgets.inactive.bg_fill =
                egui::Color32::from_rgba_unmultiplied(dark_gray, dark_gray, dark_gray, bg_opacity);
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill =
                egui::Color32::from_rgba_unmultiplied(dark_gray, dark_gray, dark_gray, bg_opacity);
            ui.style_mut().visuals.widgets.hovered.bg_fill =
                egui::Color32::from_rgba_unmultiplied(dark_gray, dark_gray, dark_gray, 80); // Slightly more opaque on hover
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill =
                egui::Color32::from_rgba_unmultiplied(dark_gray, dark_gray, dark_gray, 80);
            ui.style_mut().visuals.widgets.active.bg_fill =
                egui::Color32::from_rgba_unmultiplied(dark_gray, dark_gray, dark_gray, bg_opacity);
            ui.style_mut().visuals.widgets.active.weak_bg_fill =
                egui::Color32::from_rgba_unmultiplied(dark_gray, dark_gray, dark_gray, bg_opacity);
            ui.style_mut().visuals.widgets.noninteractive.bg_fill =
                egui::Color32::from_rgba_unmultiplied(dark_gray, dark_gray, dark_gray, bg_opacity);
            ui.style_mut().visuals.widgets.noninteractive.weak_bg_fill =
                egui::Color32::from_rgba_unmultiplied(dark_gray, dark_gray, dark_gray, bg_opacity);
            ui.style_mut().visuals.widgets.open.bg_fill =
                egui::Color32::from_rgba_unmultiplied(dark_gray, dark_gray, dark_gray, bg_opacity);
            ui.style_mut().visuals.widgets.open.weak_bg_fill =
                egui::Color32::from_rgba_unmultiplied(dark_gray, dark_gray, dark_gray, bg_opacity);

            let corner_radius = egui::CornerRadius {
                nw: 6,
                ne: 6,
                sw: 6,
                se: 6,
            };
            ui.style_mut().visuals.widgets.inactive.corner_radius = corner_radius; // Match button corner radius
            ui.style_mut().visuals.widgets.hovered.corner_radius = corner_radius;
            ui.style_mut().visuals.widgets.active.corner_radius = corner_radius;
            ui.style_mut().visuals.widgets.noninteractive.corner_radius = corner_radius;
            ui.style_mut().visuals.widgets.open.corner_radius = corner_radius;

            egui::ComboBox::from_id_salt("dns_provider")
                .selected_text(
                    egui::RichText::new(providers[current_index].0).color(egui::Color32::WHITE),
                )
                .width(BUTTON_WIDTH)
                .show_ui(ui, |ui| {
                    // Style the dropdown menu
                    ui.style_mut().visuals.override_text_color = Some(egui::Color32::WHITE);

                    for (name, provider) in providers {
                        let was_selected = matches!(
                            (name, &self.selected_provider),
                            ("Custom", DnsProvider::Custom { .. })
                        ) || std::mem::discriminant(&provider)
                            == std::mem::discriminant(&self.selected_provider);

                        if ui.selectable_label(was_selected, name).clicked() {
                            let is_custom = matches!(provider, DnsProvider::Custom { .. });
                            self.selected_provider = provider;
                            // Open custom DNS window when Custom is selected
                            if is_custom {
                                self.show_custom_dns_window = true;
                            } else {
                                // Close custom DNS window when switching away from Custom
                                self.show_custom_dns_window = false;
                            }
                        }
                    }
                });

            // Restore original styles
            ui.style_mut().spacing.button_padding = original_padding;
            ui.style_mut().visuals.widgets.inactive.bg_fill = original_bg_fill;
            ui.style_mut().visuals.widgets.inactive.corner_radius = original_corner_radius;
        });

        if matches!(self.selected_provider, DnsProvider::Custom { .. }) {
            self.selected_provider =
                DnsProvider::custom(self.custom_primary.clone(), self.custom_secondary.clone());
        }
    }

    fn render_app_state(&self, ui: &mut egui::Ui) {
        use ui_colors::{ERROR, SUCCESS, WARNING};

        match &self.app_state {
            AppState::Idle => {}
            AppState::Processing => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Processing DNS operation...");
                });
            }
            AppState::Success(message) => {
                ui.colored_label(SUCCESS, format!("✅ {}", message));
            }
            AppState::Error(message) => {
                ui.colored_label(ERROR, format!("❌ {}", message));
            }
            AppState::Warning(message) => {
                ui.colored_label(WARNING, format!("⚠️ {}", message));
            }
        }
    }

    fn render_action_buttons(&mut self, ui: &mut egui::Ui) {
        use ui_colors::{BUTTON_DANGER, BUTTON_SUCCESS, BUTTON_TEXT};
        use ui_constants::{BUTTON_HEIGHT, BUTTON_SPACING, BUTTON_WIDTH};

        ui.vertical_centered(|ui| {
            // Set DNS button (first)
            if ui
                .add_sized(
                    Vec2::new(BUTTON_WIDTH, BUTTON_HEIGHT),
                    egui::Button::new(
                        egui::RichText::new(format!(
                            "Set {} DNS",
                            self.selected_provider.display_name()
                        ))
                        .color(BUTTON_TEXT)
                        .strong() // Make text bold
                        .size(14.0), // Larger font size
                    )
                    .fill(BUTTON_SUCCESS)
                    .corner_radius(6),
                )
                .clicked()
            {
                self.handle_operation(DnsOperation::Set(self.selected_provider.clone()));
            }

            ui.add_space(BUTTON_SPACING);

            // Clear DNS button (below Set DNS)
            if ui
                .add_sized(
                    Vec2::new(BUTTON_WIDTH, BUTTON_HEIGHT),
                    egui::Button::new(
                        egui::RichText::new("Clear DNS")
                            .color(BUTTON_TEXT)
                            .strong() // Make text bold
                            .size(14.0), // Larger font size
                    )
                    .fill(BUTTON_DANGER)
                    .corner_radius(6),
                )
                .clicked()
            {
                self.show_clear_confirmation = true;
            }
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

    fn render_secondary_viewport(&mut self, ctx: &egui::Context) {
        if !self.show_second_window {
            return;
        }
        // prepare values to move into the closure (avoid capturing &mut self)
        let ping_value = self.ping_value;
        let ping_text = format!("{} ms", ping_value);
        let ping_history: Vec<f64> = self.ping_history.iter().copied().collect();
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
        let window_size = egui::vec2(400.0, 300.0); // Increased size for chart
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
                .with_title("Ping-Monitor")
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
                        // Draw ping background image with low opacity if available
                        // Use the full viewport rect to cover entire window including decorations
                        if let Some(texture) = ctx.data(|d| {
                            d.get_temp::<Option<TextureHandle>>(egui::Id::new(
                                "ping_background_texture",
                            ))
                        }) {
                            if let Some(ref tex) = texture {
                                let painter = ui.painter();
                                // Get the viewport rect which covers the entire window
                                let viewport_rect = ui.ctx().viewport_rect();
                                // Increased opacity for more visible background (0.3 = 30% opacity)
                                let tint = egui::Color32::from_rgba_unmultiplied(
                                    255,
                                    255,
                                    255,
                                    (255.0 * 0.3) as u8,
                                );
                                painter.image(
                                    tex.id(),
                                    viewport_rect,
                                    egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    tint,
                                );
                            }
                        }

                        ui.vertical(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading(" Ping Monitor");
                                ui.add_space(10.0);
                                ui.label(
                                    egui::RichText::new(ping_text.clone())
                                        .size(28.0)
                                        .color(ping_color),
                                );
                            });

                            ui.add_space(10.0);

                            // Ping history chart
                            if !ping_history.is_empty() {
                                // Color the line based on current ping value
                                let line_color = if ping_value == 0.0 {
                                    egui::Color32::LIGHT_GRAY
                                } else if ping_value < 100.0 {
                                    egui::Color32::GREEN
                                } else if ping_value < 200.0 {
                                    egui::Color32::YELLOW
                                } else {
                                    egui::Color32::RED
                                };

                                // Draw custom chart with margins
                                let chart_height = 150.0;
                                let chart_margin = 40.0; // Margin on left and right
                                let chart_width = ui.available_width() - (chart_margin * 2.0);
                                let (chart_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(chart_width, chart_height),
                                    egui::Sense::hover(),
                                );
                                // Offset the rect to add left margin
                                let chart_rect =
                                    chart_rect.translate(egui::vec2(chart_margin, 0.0));

                                let painter = ui.painter();

                                // Draw background
                                painter.rect_filled(
                                    chart_rect,
                                    0.0,
                                    egui::Color32::from_rgba_unmultiplied(20, 20, 20, 100),
                                );

                                // Find min/max for scaling
                                let min_val = ping_history
                                    .iter()
                                    .copied()
                                    .fold(f64::INFINITY, f64::min)
                                    .max(0.0);
                                let max_val = ping_history
                                    .iter()
                                    .copied()
                                    .fold(f64::NEG_INFINITY, f64::max)
                                    .max(100.0);
                                let range = (max_val - min_val).max(1.0);

                                // Draw subtle grid lines (horizontal)
                                let grid_color =
                                    egui::Color32::from_rgba_unmultiplied(150, 150, 150, 30); // Light gray, low opacity
                                for i in 0..=4 {
                                    let y =
                                        chart_rect.min.y + (chart_rect.height() / 4.0) * i as f32;
                                    painter.line_segment(
                                        [
                                            egui::pos2(chart_rect.min.x, y),
                                            egui::pos2(chart_rect.max.x, y),
                                        ],
                                        egui::Stroke::new(1.0, grid_color),
                                    );
                                }

                                // Draw subtle vertical grid lines
                                if ping_history.len() > 1 {
                                    let num_vertical_lines = (ping_history.len() - 1).min(10); // Max 10 vertical lines
                                    for i in 0..=num_vertical_lines {
                                        let x = chart_rect.min.x
                                            + (chart_rect.width() / num_vertical_lines as f32)
                                                * i as f32;
                                        painter.line_segment(
                                            [
                                                egui::pos2(x, chart_rect.min.y),
                                                egui::pos2(x, chart_rect.max.y),
                                            ],
                                            egui::Stroke::new(1.0, grid_color),
                                        );
                                    }
                                }

                                // Draw ping line
                                if ping_history.len() > 1 {
                                    let points: Vec<egui::Pos2> = ping_history
                                        .iter()
                                        .enumerate()
                                        .map(|(i, &value)| {
                                            let x = chart_rect.min.x
                                                + (chart_rect.width()
                                                    / (ping_history.len() - 1).max(1) as f32)
                                                    * i as f32;
                                            let normalized = (value - min_val) / range;
                                            let y = chart_rect.max.y
                                                - (chart_rect.height() * normalized as f32);
                                            egui::pos2(x, y)
                                        })
                                        .collect();

                                    // Draw line segments
                                    for i in 0..points.len() - 1 {
                                        painter.line_segment(
                                            [points[i], points[i + 1]],
                                            egui::Stroke::new(2.0, line_color),
                                        );
                                    }

                                    // Draw points
                                    for point in &points {
                                        painter.circle_filled(*point, 3.0, line_color);
                                    }
                                }

                                // Draw Y-axis labels
                                let label_color = egui::Color32::WHITE;
                                for i in 0..=4 {
                                    let value = max_val - (range / 4.0) * i as f64;
                                    let y =
                                        chart_rect.min.y + (chart_rect.height() / 4.0) * i as f32;
                                    painter.text(
                                        egui::pos2(chart_rect.min.x - 5.0, y),
                                        egui::Align2::RIGHT_CENTER,
                                        format!("{:.0}", value),
                                        egui::FontId::monospace(10.0),
                                        label_color,
                                    );
                                }
                            } else {
                                ui.centered_and_justified(|ui| {
                                    ui.label(
                                        egui::RichText::new("Waiting for ping data...")
                                            .color(egui::Color32::GRAY),
                                    );
                                });
                            }
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
            self.ping_history.clear();
        }
    }

    fn render_custom_dns_window(&mut self, ctx: &egui::Context) {
        if !self.show_custom_dns_window {
            return;
        }

        use ui_constants::*;

        let keep_open = std::cell::Cell::new(true);
        let window_size = egui::vec2(300.0, 240.0);
        let screen_center = ctx.input(|i| {
            let info = i.viewport();
            info.outer_rect
                .or(info.inner_rect)
                .map(|rect| rect.center())
                .unwrap_or_else(|| egui::pos2(0.0, 0.0))
        });
        let position = screen_center - window_size / 2.0;
        let viewport_id = egui::ViewportId::from_hash_of("custom_dns");

        ctx.show_viewport_immediate(
            viewport_id,
            egui::ViewportBuilder::default()
                .with_title("Custom DNS Settings")
                .with_inner_size(window_size)
                .with_position(position)
                .with_resizable(false)
                .with_decorations(true),
            {
                let keep_open = &keep_open;
                let custom_primary = &mut self.custom_primary;
                let custom_secondary = &mut self.custom_secondary;

                move |ctx, _class| {
                    if ctx.input(|i| i.viewport().close_requested()) {
                        keep_open.set(false);
                    }

                    egui::CentralPanel::default().show(ctx, |ui| {
                        // Draw custom DNS background image with low opacity if available
                        // Use the full viewport rect to cover entire window including decorations
                        if let Some(texture) = ctx.data(|d| {
                            d.get_temp::<Option<TextureHandle>>(egui::Id::new(
                                "custom_dns_background_texture",
                            ))
                        }) {
                            if let Some(ref tex) = texture {
                                let painter = ui.painter();
                                // Get the viewport rect which covers the entire window
                                let viewport_rect = ui.ctx().viewport_rect();
                                // Increased opacity for more visible background (0.3 = 30% opacity)
                                let tint = egui::Color32::from_rgba_unmultiplied(
                                    255,
                                    255,
                                    255,
                                    (255.0 * 0.3) as u8,
                                );
                                painter.image(
                                    tex.id(),
                                    viewport_rect,
                                    egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    tint,
                                );
                            }
                        }

                        ui.vertical(|ui| {
                            ui.add_space(SPACING_MEDIUM);
                            // Heading - outside the frame, centered horizontally
                            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new("Custom DNS Settings")
                                        .color(egui::Color32::WHITE)
                                        .size(18.0),
                                );
                            });
                            ui.add_space(SPACING_SMALL);

                            // Wrap everything else in a custom frame (like main window)
                            let frame = egui::Frame::group(ui.style())
                                .fill(egui::Color32::from_rgba_unmultiplied(60, 60, 65, 45)) // Same transparent blurry effect
                                .corner_radius(12.0); // Same rounded corners
                            frame.show(ui, |ui| {
                                ui.set_width(ui.available_width()); // Use full available width
                                ui.vertical(|ui| {
                                    ui.add_space(12.0);
                                    Self::render_ip_input(ui, custom_primary, "1st DNS ");
                                    ui.add_space(5.0);
                                    Self::render_ip_input(ui, custom_secondary, "2nd DNS");

                                    // Example hint text
                                    ui.add_space(3.0);
                                    ui.label(
                                        egui::RichText::new("Example: 8.8.8.8, 1.1.1.1")
                                            .color(egui::Color32::from_rgba_unmultiplied(
                                                150, 150, 150, 150,
                                            ))
                                            .size(11.0),
                                    );

                                    ui.add_space(5.0);

                                    // Buttons at bottom right
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Min),
                                        |ui| {
                                            // Close/Save button (green)
                                            use ui_colors::BUTTON_SUCCESS;
                                            if ui
                                                .add_sized(
                                                    Vec2::new(70.0, 30.0),
                                                    egui::Button::new(
                                                        egui::RichText::new("Save")
                                                            .color(egui::Color32::WHITE)
                                                            .size(12.0),
                                                    )
                                                    .fill(BUTTON_SUCCESS)
                                                    .corner_radius(6.0),
                                                )
                                                .clicked()
                                            {
                                                keep_open.set(false);
                                            }

                                            ui.add_space(5.0);

                                            // Clear button (transparent)
                                            if ui
                                                .add_sized(
                                                    Vec2::new(70.0, 30.0),
                                                    egui::Button::new(
                                                        egui::RichText::new("Clear")
                                                            .color(egui::Color32::WHITE)
                                                            .size(12.0),
                                                    )
                                                    .fill(egui::Color32::from_rgba_unmultiplied(
                                                        100, 100, 100, 100,
                                                    )) // Transparent gray
                                                    .corner_radius(6.0),
                                                )
                                                .clicked()
                                            {
                                                *custom_primary = String::new();
                                                *custom_secondary = String::new();
                                            }
                                        },
                                    );
                                    ui.add_space(5.0);
                                });
                            });
                        });
                    });
                }
            },
        );

        self.show_custom_dns_window = keep_open.get();

        // Update provider when Custom is selected (sync IPs in real-time)
        if matches!(self.selected_provider, DnsProvider::Custom { .. }) {
            self.selected_provider =
                DnsProvider::custom(self.custom_primary.clone(), self.custom_secondary.clone());
        }
    }
}

fn custom_window_frame(ctx: &egui::Context, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    use egui::{CentralPanel, UiBuilder};

    let panel_frame = egui::Frame::new()
        .fill(ctx.style().visuals.window_fill())
        .corner_radius(10)
        .outer_margin(1);

    CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
        let app_rect = ui.max_rect();

        // Draw background image with low opacity if available
        // Get texture from app data
        if let Some(texture) =
            ctx.data(|d| d.get_temp::<Option<TextureHandle>>(egui::Id::new("background_texture")))
        {
            if let Some(ref tex) = texture {
                let painter = ui.painter();
                // Increased opacity for more visible background (0.3 = 30% opacity)
                let tint =
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, (255.0 * 0.3) as u8);
                painter.image(
                    tex.id(),
                    app_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    tint,
                );
            }
        }

        use ui_constants::TITLE_BAR_HEIGHT;
        let title_bar_height = TITLE_BAR_HEIGHT;
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

fn title_bar_ui(ui: &mut egui::Ui, title_bar_rect: eframe::epaint::Rect, _title: &str) {
    use egui::{Id, PointerButton, Sense, UiBuilder, ViewportCommand};

    let title_bar_response = ui.interact(
        title_bar_rect,
        Id::new("title_bar"),
        Sense::click_and_drag(),
    );

    // Border removed - no underline under header

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
                .on_hover_text("Ping Monitor (8.8.8.8)")
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
            ui.add_space(6.0);
            minimize_button(ui);
        },
    );
}

/// Show a minimize button for the native window.
fn minimize_button(ui: &mut egui::Ui) {
    use egui::{Button, RichText, ViewportCommand};

    let button_height = 20.0;

    let minimize_resp = ui
        .add(Button::new(RichText::new("➖").size(button_height)))
        .on_hover_text("Minimize the window")
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    if minimize_resp.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
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
    // Parse IP address with proper error handling
    let target_ip = match "8.8.8.8".parse::<std::net::IpAddr>() {
        Ok(ip) => ip,
        Err(_) => return 0.0, // Return 0 on parse error
    };

    let mut p = ping::new(target_ip);
    // Reduced timeout from 2s to 1s for better responsiveness
    p.timeout(Duration::from_secs(1)).ttl(128);

    let start = Instant::now();

    match p.send() {
        Ok(_) => start.elapsed().as_millis() as f64,
        Err(_) => 0.0, // return 0 on error
    }
}
