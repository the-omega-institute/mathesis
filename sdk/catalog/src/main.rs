fn main() {
    if let Err(error) = mathesis_catalog::run_cli(std::env::args().skip(1)) {
        eprintln!("mathesis-catalog: {error}");
        std::process::exit(1);
    }
}
