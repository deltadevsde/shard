use anyhow::Result;
use std::env;

mod commands;
mod templates;
mod types;

fn print_usage() {
    println!("Usage:");
    println!("  shard init [project-name]");
    println!("  shard create-tx <tx-name> [field_name field_type]...");
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("init") => {
            let project_name = args.get(2).map(|s| s.as_str()).unwrap_or("my-rollup");
            commands::init::create_project(project_name)?;
        }
        Some("create-tx") => {
            if args.len() < 4 {
                println!("Usage: shard create-tx <tx-name> [field_name field_type]...");
                println!("Example: shard create-tx SendMessage msg String user String");
                return Ok(());
            }

            let tx_name = &args[2];
            let fields = commands::create_tx::parse_fields(&args[3..]);
            commands::create_tx::create_transaction(".", tx_name, fields)?;
        }
        _ => print_usage(),
    }

    Ok(())
}
