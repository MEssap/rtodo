mod todo_list;

use crate::todo_list::TodoList;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rtodo")]
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
    Add { description: String },
    /// List all todo items
    List {
        #[arg(short, long)]
        all: bool,
    },
    /// Complete a todo item
    Complete { id: u32 },
    /// Remove a todo item
    Remove { id: u32 },
}

fn load_todo_list(file_path: &PathBuf) -> Result<TodoList> {
    if file_path.exists() {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let todo_list = serde_json::from_reader(reader)?;
        Ok(todo_list)
    } else {
        Ok(TodoList::new())
    }
}

fn save_todo_list(file_path: &PathBuf, todo_list: &TodoList) -> Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;

    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, todo_list)?;
    Ok(())
}

fn expand_path(path: &String) -> Result<PathBuf> {
    if path.starts_with('~') {
        let home_dir = env::var("HOME").context("HOME environment variable not set")?;

        if path == "~" {
            Ok(PathBuf::from(home_dir))
        } else {
            let stripped_path = path.trim_start_matches('~').trim_start_matches('/');
            Ok(PathBuf::from(home_dir).join(stripped_path))
        }
    } else {
        Ok(PathBuf::from(path))
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let file_path = expand_path(&cli.file)?;
    let mut todo_list = load_todo_list(&file_path)?;

    match cli.command {
        Commands::Add { description } => {
            let item = todo_list.add_item(description)?;
            println!("Added todo item #{}: {}", item.id, item.description);
        }
        Commands::List { all } => {
            let items = todo_list.list_items(all);
            if items.is_empty() {
                println!("No todo items found.");
            } else {
                println!("Todo List({}):", todo_list.todo_len());
                for item in items {
                    let status = if item.completed { "✓" } else { " " };
                    println!("[{}] #{}: {}", status, item.id, item.description);
                }
            }
        }
        Commands::Complete { id } => {
            let item = todo_list.complete_item(id)?;
            println!("Completed todo item #{}: {}", item.id, item.description);
        }
        Commands::Remove { id } => {
            let item = todo_list.remove_item(id)?;
            println!("Removed todo item #{}: {}", item.id, item.description);
        }
    }

    save_todo_list(&file_path, &todo_list)
}
