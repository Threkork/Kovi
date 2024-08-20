use clap::{Parser, Subcommand};
use cmd::{add::add, new_kovi::new_kovi, new_plugin::new_plugin};

mod cmd;


#[derive(Parser, Debug)]
#[command(version, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: CMDs,
}


#[derive(Debug, Subcommand)]
enum CMDs {
    #[command(alias = "c")]
    Create { name: String },

    #[command(alias = "n")]
    New {
        #[arg(default_value = "kovi-bot")]
        name: String,

        #[arg(short, long)]
        version: Option<String>,
    },

    #[command(alias = "a")]
    Add { name: String },
}


fn main() {
    let args = Args::parse();

    match args.command {
        CMDs::Create { name } => new_plugin(name),
        CMDs::New { name, version } => new_kovi(name, version),
        CMDs::Add { name } => add(name),
    }
}
