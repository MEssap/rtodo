use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::atomic::Ordering};

use crate::SHOW_COMPLETE;

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: usize,
    pub description: String,
    pub completed: bool,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub sub_list: Option<TodoList>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IdPool {
    next_id: usize,
    recycled_ids: Vec<usize>,
    used_ids: HashSet<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
    id_pool: IdPool,
}

impl IdPool {
    /// Creates a new IdPool
    fn new() -> Self {
        Self {
            next_id: 0,
            recycled_ids: Vec::new(),
            used_ids: HashSet::new(),
        }
    }

    /// Acquires a new ID, reusing recycled IDs when available
    fn acquire_id(&mut self) -> usize {
        if let Some(id) = self.recycled_ids.pop() {
            self.used_ids.insert(id);
            id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            self.used_ids.insert(id);
            id
        }
    }

    /// Releases an ID back to the pool for reuse
    fn release_id(&mut self, id: usize) -> Result<()> {
        if !self.used_ids.contains(&id) {
            return Err(anyhow::anyhow!("ID {} is not in use", id));
        }

        self.used_ids.remove(&id);
        self.recycled_ids.push(id);
        Ok(())
    }
}

impl Default for TodoList {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoList {
    /// Creates a new empty TodoList
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            id_pool: IdPool::new(),
        }
    }

    /// Parses a path string to navigate to a specific TodoItem
    /// 
    /// Path format: "0" for top level item, "0:1:2" for nested items
    fn parse_path(&mut self, path: &String) -> Result<&mut TodoItem> {
        let indices: Vec<usize> = path
            .split(':')
            .map(|s| s.parse::<usize>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| anyhow::anyhow!("Invalid path format: {}", path))?;

        if indices.is_empty() {
            return Err(anyhow::anyhow!("Not found path."));
        }

        let mut current_list = self;
        for (depth, &index) in indices.iter().enumerate() {
            if index >= current_list.items.len() {
                return Err(anyhow::anyhow!(
                    "Index {} out of bounds at depth {} (path: {})",
                    index,
                    depth,
                    path
                ));
            }

            if depth == indices.len() - 1 {
                return Ok(&mut current_list.items[index]);
            }

            let item = &mut current_list.items[index];
            current_list = item
                .sub_list
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("No sublist at index {} (path: {})", index, path))?;
        }

        unreachable!()
    }

    /// Creates a new TodoItem and adds it to the list or a sublist
    /// 
    /// # Arguments
    /// * `description` - Description of the todo item
    /// * `deadline` - Optional deadline for the todo item
    /// * `parent_path` - Optional path to parent item for creating subtasks
    pub fn add_item(
        &mut self,
        description: String,
        deadline: Option<DateTime<Local>>,
        parent_path: Option<&String>,
    ) -> Result<&TodoItem> {
        // get sublist or create a new one
        let list = if let Some(p) = parent_path {
            let parent = self.parse_path(p)?;
            parent.sub_list.get_or_insert(TodoList::new())
        } else {
            self
        };
        let id = list.id_pool.acquire_id();
        let time = deadline.map(|deadline| deadline.to_string());
        let item = TodoItem {
            id,
            description,
            completed: false,
            deadline: time,
            sub_list: None,
        };
        list.items.push(item);
        list.items
            .last()
            .ok_or(anyhow::anyhow!("Cannot get todolist item"))
    }

    /// Edits an existing TodoItem at the specified path
    /// 
    /// # Arguments
    /// * `path` - Path to the item to edit
    /// * `description` - New description for the todo item
    /// * `deadline` - Optional new deadline for the todo item
    pub fn edit_item(
        &mut self,
        path: &String,
        description: String,
        deadline: Option<DateTime<Local>>,
    ) -> Result<&TodoItem> {
        let item = self.parse_path(path)?;
        item.description = description;
        item.deadline = deadline.map(|deadline| deadline.to_string());
        Ok(item)
    }

    /// Returns a list of TodoItems based on SHOW_COMPLETE flag
    /// 
    /// If SHOW_COMPLETE is true, returns all items; otherwise returns only incomplete items
    pub fn list_items(&self) -> Vec<&TodoItem> {
        if SHOW_COMPLETE.load(Ordering::SeqCst) {
            self.items.iter().collect()
        } else {
            self.items.iter().filter(|item| !item.completed).collect()
        }
    }

    /// Marks a TodoItem as completed at the specified path
    pub fn complete_item(&mut self, path: &String) -> Result<&TodoItem> {
        let item = self.parse_path(path)?;
        item.complete();
        Ok(item)
    }

    /// Removes a TodoItem at the specified path and returns it
    /// 
    /// Also releases the item's ID back to the ID pool for reuse
    pub fn remove_item(&mut self, path: &str) -> Result<TodoItem> {
        /// Splits parent and child path from a colon-separated string
        fn split_path(path: &str) -> Result<(Option<String>, usize)> {
            if let Some((parent_str, child_str)) = path.rsplit_once(":") {
                let child = child_str.parse::<usize>().context("Invalid parse format")?;
                Ok((Some(parent_str.into()), child))
            } else {
                // If no colon, treat the entire path as the child ID at root level
                let child = path.parse::<usize>().context("Invalid parse format")?;
                Ok((None, child))
            }
        }

        let (parent_path, id) = split_path(path)?;
        let parent = if let Some(p) = parent_path {
            self.parse_path(&p)?.sub_list.get_or_insert(TodoList::new())
        } else {
            self
        };
        let index = parent
            .items
            .iter()
            .position(|item| item.id == id)
            .ok_or_else(|| anyhow::anyhow!("Item with id {} not found", id))?;

        parent.id_pool.release_id(id)?;
        Ok(parent.items.remove(index))
    }

    /// Returns the count of incomplete todo items
    pub fn todo_len(&self) -> usize {
        self.items.iter().filter(|item| !item.completed).count()
    }
}

impl TodoItem {
    /// Marks this TodoItem as completed
    pub fn complete(&mut self) {
        self.completed = true;
    }

    /// Displays the TodoItem with proper formatting and indentation
    /// 
    /// # Arguments
    /// * `depth` - Indentation depth for nested items
    pub fn display(&self, depth: usize) {
        let status = if self.completed { " | ✓" } else { "" };
        println!(
            "{}#{}: {}{}{}{}",
            "  ".repeat(depth),
            self.id,
            self.description,
            match &self.sub_list {
                Some(list) => format!("({})", list.todo_len()),
                None => String::new(),
            },
            match &self.deadline {
                Some(time) => format!(" | deadline: {}", time),
                None => String::new(),
            },
            status,
        );
        if let Some(sub_list) = &self.sub_list {
            let items = sub_list.list_items();
            for item in items {
                item.display(depth + 1);
            }
        };
    }
}

#[cfg(test)]
mod todo_list_tests {
    use super::*;
    use crate::utils::{expand_path, parse_deadline, save_todo_list};

    #[test]
    fn create() -> Result<()> {
        let mut list = TodoList::new();
        list.add_item("test1".to_string(), None, None)?;
        list.add_item("test2".to_string(), None, None)?;
        let time = parse_deadline(Some("today".to_string()))?;
        list.add_item("test3".to_string(), Some(time), None)?;
        list.add_item("test4".to_string(), None, None)?;
        // list.add_item("test5".to_string(), None, Some(0))?;
        let path_str = "~/.todo".to_string();
        let path = expand_path(&path_str)?;

        save_todo_list(&path, &list)?;

        Ok(())
    }
}
