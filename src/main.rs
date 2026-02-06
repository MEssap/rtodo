mod todo_list;
mod utils;

use crate::utils::{expand_path, load_todo_list, parse_deadline, save_todo_list};
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::aot::{Bash, Elvish, Fish, PowerShell, Zsh};
use clap_complete::Shell;
use clap_complete::{generate, Generator};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Parser)]
#[command(name = "td")]
#[command(about = "A simple todo list manager in rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, default_value = "~/.todo")]
    file: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new todo item
    Add {
        /// Description of the todo item
        description: String,
        /// Deadline of todo item
        #[arg(short, long)]
        deadline: Option<String>,
        /// Sub todo list of parent id
        #[arg(short, long)]
        parent_path: Option<String>,
    },
    /// Edit todo item with id
    Edit {
        /// Sub todolist of parent id
        path: String,
        /// Description of the todo item
        description: String,
        /// Deadline of todo item
        #[arg(short, long)]
        deadline: Option<String>,
    },
    /// List all todo items
    List {
        #[arg(short, long)]
        all: bool,
    },
    /// Complete a todo item
    Complete {
        /// Sub todolist of parent id
        path: String,
    },
    /// Remove a todo item
    Remove {
        /// Sub todo list of parent id
        path: String,
    },
    /// Generate shell completion scripts
    Completion {
        /// Shell type to generate completion for
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn print_completion<G: Generator>(generator: G, cmd: &mut clap::Command) {
    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut io::stdout(),
    );
}

static SHOW_COMPLETE: AtomicBool = AtomicBool::new(false);

fn main() -> Result<()> {
    let cli = Cli::parse();
    let file_path = expand_path(&cli.file)?;
    let mut todo_list = load_todo_list(&file_path)?;

    match cli.command {
        Commands::Add {
            description,
            deadline,
            parent_path,
        } => {
            let deadline = parse_deadline(deadline).ok();
            let item = todo_list.add_item(description, deadline, parent_path.as_ref())?;
            println!(
                "Added todo item #{}{}: {}",
                parent_path.map_or(String::new(), |path| format!("{}:", path)),
                item.id,
                item.description
            );
        }
        Commands::Edit {
            path,
            description,
            deadline,
        } => {
            let deadline = parse_deadline(deadline).ok();
            let item = todo_list.edit_item(&path, description, deadline)?;
            println!(
                "Edit todo item #{}: {} {}",
                path,
                item.description,
                match deadline {
                    Some(time) => format!("| deadline: {}", time),
                    None => String::new(),
                }
            );
        }
        Commands::List { all } => {
            SHOW_COMPLETE.store(all, Ordering::SeqCst);
            let items = todo_list.list_items();
            if items.is_empty() {
                println!("No todo items found.");
            } else {
                println!("Todo List({}):", todo_list.todo_len());
                items.iter().for_each(|i| i.display(0));
            }
        }
        Commands::Complete { path } => {
            let item = todo_list.complete_item(&path)?;
            println!("Completed todo item #{}: {}", path, item.description);
        }
        Commands::Remove { path } => {
            let item = todo_list.remove_item(&path)?;
            println!("Removed todo item #{}: {}", path, item.description);
        }
        Commands::Completion { shell } => {
            let mut cmd = Cli::command();
            match shell {
                Shell::Bash => print_completion(Bash, &mut cmd),
                Shell::Elvish => print_completion(Elvish, &mut cmd),
                Shell::Fish => print_completion(Fish, &mut cmd),
                Shell::PowerShell => print_completion(PowerShell, &mut cmd),
                Shell::Zsh => print_completion(Zsh, &mut cmd),
                _ => {
                    todo!()
                }
            }
        }
    }

    save_todo_list(&file_path, &todo_list)
}
