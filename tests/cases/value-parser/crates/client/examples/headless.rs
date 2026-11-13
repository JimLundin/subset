#[derive(Parser)]
struct Args {
    #[arg(long, value_parser = ["claude", "codex"])]
    provider: String,
}
