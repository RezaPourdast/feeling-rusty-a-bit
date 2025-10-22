use std::io::{self, Read};
fn main() {
    println!("Hello, world!");
    println!("Matin Afzal is here!");

    let mut buffer=[0;1];
    io::stdin().read_exact(&mut buffer).expect("!");
}