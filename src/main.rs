use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;


#[derive(Parser, Debug)]
#[clap(author, about, long_about = None)]
struct Cli {
    /// Comando a ejecutar
    #[command(subcommand)]
    command: Command,
    
}

#[derive(Subcommand, Debug)]
enum Command {
    Organize {

        path: Option<PathBuf>,

        #[arg(short, long = "dry-run")]
        dry_run: bool,

        #[arg(short, long)]
        output: Option<PathBuf>,

        #[arg(short, long = "keep-original")]
        keep_original: bool,

        #[arg(short, long, default_value = "extension")]
        by: String,

        #[arg(short, long, default_value = "false")]
        recursive: bool,
    },
    Preview {
        path: Option<PathBuf>,
        #[arg(short, long, default_value = "extension")]
        by: String,
    },
    #[command(name = "add-rule")]
    AddRule {
        category: String,

        #[arg(short, long,  num_args = 1..)]
        extensions: Vec<String>
    },

    #[command(name = "remove-rule")]
    RemoveRule {
        category: String,

        #[arg(short, long,  num_args = 1..)]
        extensions: Option<Vec<String>>
    },
    Rules,
    Undo

}

fn main() {
    // Run the command and turn any error into a friendly message + non-zero exit
    // code instead of a panic. `{err:#}` prints the whole anyhow context chain.
    if let Err(err) = run() {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Command::Organize { path, dry_run, output, keep_original, by, recursive } => {
            commands::organize_files(path, dry_run, &output, keep_original, &by, recursive)?;
        }
        Command::Preview { path, by } => {
            commands::organize_files(path, true, &None, false, &by, false)?;
        }
        Command::AddRule { category, extensions } => {
            commands::save_rule(category, extensions)?;
        }
        Command::RemoveRule { category, extensions } => {
            commands::remove_rule(category, extensions)?;
        }
        Command::Rules => {
            commands::show_rules()?;
        }
        Command::Undo => {
            let history_path = commands::get_history_path()?;
            commands::undo(&history_path)?;
        }
    }

    Ok(())
}