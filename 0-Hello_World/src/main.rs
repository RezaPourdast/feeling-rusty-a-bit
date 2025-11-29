use std::io::{self, Read};
fn main() {
    println!("Hello, world!");
    println!("Reza Pourdast is here!");

    let mut buffer=[0;1];
    io::stdin().read_exact(&mut buffer).expect("!");
}
