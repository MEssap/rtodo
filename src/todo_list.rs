use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: u32,
    pub description: String,
    pub completed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct IdPool {
    next_id: u32,
    recycled_ids: Vec<u32>,
    used_ids: HashSet<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
    id_pool: IdPool,
}

impl IdPool {
    fn new() -> Self {
        Self {
            next_id: 1,
            recycled_ids: Vec::new(),
            used_ids: HashSet::new(),
        }
    }

    /// 获取ID
    fn acquire_id(&mut self) -> u32 {
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

    /// 释放ID
    fn release_id(&mut self, id: u32) -> Result<()> {
        if !self.used_ids.contains(&id) {
            return Err(anyhow::anyhow!("ID {} is not in use", id));
        }

        self.used_ids.remove(&id);
        self.recycled_ids.push(id);
        Ok(())
    }
}

impl TodoList {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            id_pool: IdPool::new(),
        }
    }

    pub fn add_item(&mut self, description: String) -> Result<&TodoItem> {
        let id = self.id_pool.acquire_id();
        let item = TodoItem {
            id: id,
            description,
            completed: false,
        };
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
        self.id_pool.release_id(id)?;

        Ok(self.items.remove(index))
    }

    pub fn todo_len(&self) -> usize {
        self.items.iter().filter(|item| !item.completed).count()
    }
}
