mod todo_list;
mod utils;

use crate::utils::{expand_path, load_todo_list, parse_deadline, save_todo_list};
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use clap_complete::aot::{Bash, Elvish, Fish, PowerShell, Zsh};
use clap_complete::{Generator, generate};
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
        id: usize,
        /// Description of the todo item
        description: String,
        /// Deadline of todo item
        #[arg(short, long)]
        deadline: Option<String>,
        /// Sub todolist of parent id
        #[arg(short, long)]
        parent_path: Option<String>,
    },
    /// List all todo items
    List {
        #[arg(short, long)]
        all: bool,
    },
    /// Complete a todo item
    Complete {
        id: usize,
        /// Sub todolist of parent id
        #[arg(short, long)]
        parent_path: Option<String>,
    },
    /// Remove a todo item
    Remove {
        id: usize,
        /// Sub todo list of parent id
        #[arg(short, long)]
        parent_path: Option<String>,
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
                "Added todo item #{:?}:{}: {}",
                parent_path, item.id, item.description
            );
        }
        Commands::Edit {
            id,
            description,
            deadline,
            parent_path,
        } => {
            let deadline = parse_deadline(deadline).ok();
            let item = if let Some(p) = &parent_path {
                todo_list
                    .parse_path(p)?
                    .sub_list
                    .as_mut()
                    .ok_or(anyhow::anyhow!("Parent not found"))?
                    .items
                    .get_mut(id)
            } else {
                todo_list.items.get_mut(id)
            }
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Cannot get edit item.",
            ))?;

            item.description = description.clone();
            if let Some(time) = deadline {
                item.deadline = Some(time.to_string());
            }

            println!(
                "Edit todo item #{:?}:{}: {} {}",
                parent_path,
                id,
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
        Commands::Complete { id, parent_path } => {
            let item = if let Some(p) = &parent_path {
                todo_list
                    .parse_path(p)?
                    .sub_list
                    .as_mut()
                    .ok_or(anyhow::anyhow!("Parent or sublist not found"))?
                    .complete_item(id)?
            } else {
                todo_list.complete_item(id)?
            };

            println!(
                "Completed todo item #{:?}:{}: {}",
                parent_path, item.id, item.description
            );
        }
        Commands::Remove { id, parent_path } => {
            let item = if let Some(p) = &parent_path {
                todo_list
                    .parse_path(p)?
                    .sub_list
                    .as_mut()
                    .ok_or(anyhow::anyhow!("Parent or sublist not found"))?
                    .remove_item(id)?
            } else {
                todo_list.remove_item(id)?
            };

            println!(
                "Removed todo item #{:?}:{}: {}",
                parent_path, item.id, item.description
            );
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
