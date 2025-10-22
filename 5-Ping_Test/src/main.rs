#![windows_subsystem = "windows"]

use ping;
use sdl2::event::Event;
use sdl2::image::{InitFlag, LoadTexture};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use sdl2::ttf::Font;
use std::time::Instant;

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().unwrap();
    let font_path = "assets/Roboto-Medium.ttf";
    let font = ttf_context.load_font(font_path, 32)?;

    let window = video_subsystem
        .window("Ping Test", 500, 500)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let mut event_pump = sdl_context.event_pump()?;

    let _image_context = sdl2::image::init(InitFlag::PNG)?;
    let texture_creator = canvas.texture_creator();
    let texture = texture_creator.load_texture("assets/globe_.png")?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        canvas.clear();
        canvas.copy(&texture, None, None)?;
        get_ping(&mut canvas, &texture_creator, &font);
        canvas.present();
    }

    Ok(())
}

fn get_ping(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    texture_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    font: &Font,
) {
    let target_ip = "8.8.8.8".parse().unwrap();
    let mut p = ping::new(target_ip);
    p.timeout(std::time::Duration::from_secs(2)).ttl(128);

    let start = Instant::now();

    let text = match p.send() {
        Ok(_) => {
            let rtt = (start.elapsed().as_secs_f64() * 1000.0) as u64;
            format!("Ping: {} ms", rtt)
        }
        Err(e) => format!("Ping failed: {}", e),
    };

    let surface = font
        .render(&text)
        .blended(Color::RGB(255, 255, 255))
        .unwrap();
    let text_texture = texture_creator
        .create_texture_from_surface(&surface)
        .unwrap();
    let TextureQuery { width, height, .. } = text_texture.query();
    let (window_width, window_height) = canvas.output_size().unwrap();
    let x = window_width as i32 / 2 - width as i32 / 2;
    let y = window_height as i32 / 2 - height as i32 / 2;

    canvas
        .copy(&text_texture, None, Some(Rect::new(x, y, width, height)))
        .unwrap();
}
