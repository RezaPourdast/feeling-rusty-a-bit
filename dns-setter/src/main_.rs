//! DNS Setter - Windows GUI for managing DNS settings

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Vec2};
use regex::Regex;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

const CREATE_NO_WINDOW: u32 = 0x08000000; // Hide console window

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 600.0]),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "DNS SETTER",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::default())
        }),
    )
}

#[derive(Default)]
struct MyApp {
    adapter: Option<String>,
    dns: Vec<String>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DNS SETTER");
            if self.dns.is_empty() {
                ui.label("No DNS Found.");
            } else {
                ui.label(format!("Current DNS: {}", self.dns.join(", ")));
            }

            ui.vertical_centered(|ui| {
                let available_height = ui.available_height();
                ui.add_space(available_height / 2.0 - 50.0);

                if ui
                    .add_sized(
                        Vec2::new(120.0, 50.0),
                        egui::Button::new("electro dns").corner_radius(10),
                    )
                    .clicked()
                {
                    self.adapter = get_active_adapter();
                    if let Some(adapter) = &self.adapter {
                        set_dns(adapter, "78.157.42.100", "78.157.42.101");
                        self.dns = get_current_dns(adapter);
                    } else {
                        println!("No Internet Connection Found.");
                    }
                }

                ui.add_space(10.0);

                if ui
                    .add_sized(
                        Vec2::new(120.0, 50.0),
                        egui::Button::new("clear dns").corner_radius(10),
                    )
                    .clicked()
                {
                    self.adapter = get_active_adapter();
                    if let Some(adapter) = &self.adapter {
                        self.dns = get_current_dns(adapter);
                        if !self.dns.is_empty() {
                            clear_dns(adapter);
                            self.dns = get_current_dns(adapter);
                        } else {
                            println!("error clearing dns.");
                        }
                    } else {
                        println!("No Internet Connection Found.");
                    }
                }

                ui.add_space(10.0);

                // ui.add_space(10.0);
                // ui.image(egui::include_image!("../asset/cat.webp"))
            });
        });
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
