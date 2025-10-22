#![windows_subsystem = "windows"]

use ping;
use sdl2::event::Event;
use sdl2::image::{InitFlag, LoadTexture};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().unwrap();
    let font = ttf_context.load_font("assets/Roboto-Medium.ttf", 32)?;
    let small_font = ttf_context.load_font("assets/Roboto-Medium.ttf", 24)?;

    let window = video_subsystem
        .window("Ping Test", 600, 600)
        .resizable()
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let _image_context = sdl2::image::init(InitFlag::PNG)?;
    let texture_creator = canvas.texture_creator();
    let texture = texture_creator.load_texture("assets/globe.png")?;

    let current_ping = Arc::new(Mutex::new(String::from("Ping: ...")));
    let rtt_history = Arc::new(Mutex::new(VecDeque::with_capacity(5)));

    {
        let current_clone = Arc::clone(&current_ping);
        let hist_clone = Arc::clone(&rtt_history);
        thread::spawn(move || ping_thread(current_clone, hist_clone));
    }

    'running: loop {
        for event in event_pump.poll_iter() {
            if let Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } = event
            {
                break 'running;
            }
        }

        canvas.set_draw_color(Color::RGB(5, 16, 28));
        canvas.clear();
        canvas.copy(&texture, None, None)?;

        draw_current_ping(&mut canvas, &texture_creator, &font, &current_ping);
        draw_ping_history(&mut canvas, &texture_creator, &small_font, &rtt_history);

        canvas.present();

        std::thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}

fn ping_thread(current_ping: Arc<Mutex<String>>, rtt_history: Arc<Mutex<VecDeque<String>>>) {
    let target_ip = "8.8.8.8".parse().unwrap();
    let mut p = ping::new(target_ip);
    p.timeout(Duration::from_secs(1)).ttl(128);

    loop {
        let start = Instant::now();
        let rtt: Option<u64> = match p.send() {
            Ok(_) => Some((start.elapsed().as_secs_f64() * 1000.0) as u64),
            Err(_) => None,
        };

        if let Ok(mut hist) = rtt_history.try_lock() {
            if hist.len() >= 5 {
                hist.pop_front();
            }
            hist.push_back(match rtt {
                Some(ms) => format!("{} ms", ms),
                None => "Ping failed".to_string(),
            });
        }

        if let Ok(mut current) = current_ping.try_lock() {
            *current = match rtt {
                Some(ms) => format!("Current Ping: {} ms", ms),
                None => "Ping failed".to_string(),
            };
        }

        thread::sleep(Duration::from_secs(1));
    }
}

fn draw_current_ping(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    texture_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    font: &sdl2::ttf::Font,
    current_ping: &Arc<Mutex<String>>,
) {
    let text = current_ping.lock().unwrap().clone();

    let rtt_ms: u64 = text
        .trim_start_matches("Current Ping: ")
        .trim_end_matches(" ms")
        .parse()
        .unwrap_or(9999);

    let color = if rtt_ms < 100 {
        Color::RGB(0, 255, 0)
    } else if rtt_ms < 150 {
        Color::RGB(255, 255, 0)
    } else {
        Color::RGB(255, 0, 0)
    };

    let surface = font.render(&text).blended(color).unwrap();
    let text_texture = texture_creator
        .create_texture_from_surface(&surface)
        .unwrap();
    let TextureQuery { width, height, .. } = text_texture.query();
    let (window_width, _) = canvas.output_size().unwrap();
    let x = window_width as i32 / 2 - width as i32 / 2;
    let y = 100;

    canvas
        .copy(&text_texture, None, Some(Rect::new(x, y, width, height)))
        .unwrap();
}

fn draw_ping_history(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    texture_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    font: &sdl2::ttf::Font,
    rtt_history: &Arc<Mutex<VecDeque<String>>>,
) {
    let history = rtt_history.lock().unwrap();
    let (window_width, _) = canvas.output_size().unwrap();

    let mut y = 250;
    for text in history.iter().rev() {
        let color = if text.contains("failed") {
            Color::RGB(255, 0, 0)
        } else {
            let ms_value: u64 = text
                .split_whitespace()
                .next()
                .unwrap_or("9999")
                .parse()
                .unwrap_or(9999);

            if ms_value < 100 {
                Color::RGB(0, 255, 0)
            } else if ms_value < 150 {
                Color::RGB(255, 255, 0)
            } else {
                Color::RGB(255, 0, 0)
            }
        };

        let surface = font.render(text).blended(color).unwrap();
        let text_texture = texture_creator
            .create_texture_from_surface(&surface)
            .unwrap();
        let TextureQuery { width, height, .. } = text_texture.query();
        let x = (window_width as i32 / 2) - (width as i32 / 2);
        canvas
            .copy(&text_texture, None, Some(Rect::new(x, y, width, height)))
            .unwrap();

        y += height as i32 + 5;
    }
}
