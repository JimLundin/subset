#[derive(Parser)]
struct Args {
    #[arg(long, value_enum)]
    provider: Provider,
}
