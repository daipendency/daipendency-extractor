use laibrary::generate_library_api;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: laibrary <path-to-library>");
        std::process::exit(1);
    }

    let library_path = Path::new(&args[1]);
    match generate_library_api(library_path) {
        Ok(output) => println!("{}", output),
        Err(e) => eprintln!("Error: {}", e),
    }
}
