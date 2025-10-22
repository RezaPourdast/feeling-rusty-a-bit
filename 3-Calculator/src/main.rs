use std::io;
use std::io::Read;
use std::io::Write;

fn main() {
    let mut run = true;
    let mut number1: f32;
    let mut number2: f32;
    let mut operation: String;
    let mut stay: String;

    while run {
        clear_console();

        print!("\nEnter number one: ");
        io::stdout().flush().unwrap();
        let mut tmp_input = String::new();
        io::stdin().read_line(&mut tmp_input).expect("!");
        number1 = tmp_input.trim().parse().expect("!");

        print!("\nEnter number two: ");
        io::stdout().flush().unwrap();
        tmp_input.clear();
        io::stdin().read_line(&mut tmp_input).expect("!");
        number2 = tmp_input.trim().parse().expect("!");

        print!("\nSelect your operation: [+, -, *, /] ");
        io::stdout().flush().unwrap();
        tmp_input.clear();
        io::stdin().read_line(&mut tmp_input).expect("!");
        operation = tmp_input.trim().to_string();

        if operation == "+" {
            print!("\n{} + {} = {}\n", number1, number2, number1 + number2);
            io::stdout().flush().unwrap();
        }
        else if operation == "-" {
            print!("\n{} - {} = {}\n", number1, number2, number1 - number2);
            io::stdout().flush().unwrap();
        }
        else if operation == "*" {
            print!("\n{} * {} = {}\n", number1, number2, number1 * number2);
            io::stdout().flush().unwrap();
        }
        else if operation == "/" {
            print!("\n{} / {} = {}\n", number1, number2, number1 / number2);
            io::stdout().flush().unwrap();
        }

        print!("\nDo you want to continue? [y/n] ");
        io::stdout().flush().unwrap();
        tmp_input.clear();
        io::stdin().read_line(&mut tmp_input).expect("!");
        stay = tmp_input.trim().to_string();

        if stay.to_lowercase() == "n" {
            run = false;
        }
    }

    clear_console();
    println!("Program ends!\nPress any key to exit!");
    let mut buffer = [0;1];
    io::stdin().read_exact(&mut buffer).expect("!");
}

fn clear_console() {
    print!("{}[2J{}[H", 27 as char, 27 as char);
}