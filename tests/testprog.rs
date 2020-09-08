use std::process::exit;

fn main() {
    println!("test-stdout");
    eprintln!("test-stderr");
    exit(1);
}
