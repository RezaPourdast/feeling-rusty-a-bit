use std::io::{self, Read};

fn main() {
	for i in 1..11 {
		for j in 1..11 {
			print!("{} * {} = {}\t", i, j, i * j);
		}
		print!("\n");
	}

    let mut buffer = [0;1];
    io::stdin().read_exact(&mut buffer).expect("!");
}