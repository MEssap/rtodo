use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: u32,
    pub description: String,
    pub completed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
    pub next_id: u32,
}

impl TodoList {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add_item(&mut self, description: String) -> Result<&TodoItem> {
        let item = TodoItem {
            id: self.next_id,
            description,
            completed: false,
        };
        self.next_id += 1;
        self.items.push(item);
        self.items
            .last()
            .ok_or(anyhow::anyhow!("Cannot get todolist item"))
    }

    pub fn list_items(&self, show_completed: bool) -> Vec<&TodoItem> {
        if show_completed {
            self.items.iter().collect()
        } else {
            self.items.iter().filter(|item| !item.completed).collect()
        }
    }

    pub fn complete_item(&mut self, id: u32) -> Result<&TodoItem> {
        let item = self
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or(anyhow::anyhow!("Item with id {} not found", id))?;

        item.completed = true;
        Ok(item)
    }

    pub fn remove_item(&mut self, id: u32) -> Result<TodoItem> {
        let index = self
            .items
            .iter()
            .position(|item| item.id == id)
            .ok_or_else(|| anyhow::anyhow!("Item with id {} not found", id))?;

        Ok(self.items.remove(index))
    }
}
