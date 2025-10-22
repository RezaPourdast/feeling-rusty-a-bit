use std::io;
use std::io::*;
use minifb::*;
use image::*;

fn main() {
  let mut path = String::new();
  print!("Enter your image path: ");
  io::stdout().flush().unwrap();
  io::stdin().read_line(&mut path).expect("!");
  let image_path = path.trim();

  let image = match open(image_path) { // open is from image
    Ok(image) => image,
    Err(e) => {
      eprintln!("Failed! {}", e);
      wait_to_see_error();
      return;
    }
  };

  let image = image.to_rgba8(); // Add alpah channel
  let (width, height) = image.dimensions();

  let buffer: Vec<u32> = image.pixels().map(|px|
    {
      let r = px.0[0];
      let g = px.0[1];
      let b = px.0[2];
      let a = px.0[3];
      (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | (b as u32)
    }).collect();

  let mut screen = match Window::new("4-Image_Viewer", width as usize, height as usize, WindowOptions::default()) {
    Ok(screen) => screen,
    Err(e) => {
      eprintln!("Failed! {}", e);
      wait_to_see_error();
      return;
    }
  };

  while screen.is_open() && !screen.is_key_down(minifb::Key::Escape) {
    screen.update_with_buffer(&buffer, width as usize, height as usize).unwrap();
  }
}

fn wait_to_see_error() {
  let mut buffer = [0;1];
  print!("Program ends!\nPress any key to exit!");
  io::stdout().flush().unwrap();
  io::stdin().read_exact(&mut buffer);
}