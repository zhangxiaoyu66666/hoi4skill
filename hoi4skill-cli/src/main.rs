fn main() {
    if let Err(err) = hoi4skill::cli::run(std::env::args().skip(1).collect()) {
        eprintln!("ERROR: {err}");
        std::process::exit(1);
    }
}
