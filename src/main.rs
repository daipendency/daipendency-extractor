use laibrary::generate_documentation;
use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <language> <path>", args[0]);
        std::process::exit(1);
    }

    let language = &args[1];
    let path = PathBuf::from(&args[2]);

    match generate_documentation(language, &path) {
        Ok(output) => println!("{}", output),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }
    }
}
