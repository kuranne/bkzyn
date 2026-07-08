use clap::{Parser, Subcommand};
#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}
#[derive(Subcommand, Debug)]
enum Cmd {
    Add {
        paths: Vec<String>,
        #[arg(short, long)]
        ignore: Option<Vec<String>>,
    }
}
fn main() {
    let args = vec!["bkzyn", "add", "nvim", "--ignore", "*.json", "LICENSE", "README.md"];
    let cli = Cli::parse_from(args);
    println!("{:#?}", cli);
}
