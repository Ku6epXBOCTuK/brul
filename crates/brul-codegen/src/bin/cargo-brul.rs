fn main() -> Result<(), brul_codegen::cli::Error> {
    brul_codegen::cli::run(std::env::args().skip(1))
}
