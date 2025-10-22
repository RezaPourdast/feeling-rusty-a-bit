use std::path::Path;

fn main() {
    let sdl2_dir = Path::new("C:\\sdl\\SDL2-2.26.5");
    let sdl2_ttf_dir = Path::new("C:\\sdl\\SDL2_ttf-2.24.0");
    let sdl2_image_dir = Path::new("C:\\sdl\\SDL2_image-2.6.5");

    println!(
        "cargo:rustc-link-search=native={}",
        sdl2_dir.join("lib\\x64").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        sdl2_ttf_dir.join("lib\\x64").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        sdl2_image_dir.join("lib\\x64").display()
    );

    println!("cargo:rustc-link-lib=SDL2");
    println!("cargo:rustc-link-lib=SDL2_image");
    println!("cargo:rustc-link-lib=SDL2_ttf");

    println!(
        "cargo:rerun-if-changed={}",
        sdl2_dir.join("lib\\x64").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sdl2_ttf_dir.join("lib\\x64").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sdl2_image_dir.join("lib\\x64").display()
    );
}
