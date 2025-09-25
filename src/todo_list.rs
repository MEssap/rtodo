use anyhow::Result;
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
    fn new() -> Self {
        Self {
            next_id: 0,
            recycled_ids: Vec::new(),
            used_ids: HashSet::new(),
        }
    }

    /// 获取ID
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

    /// 释放ID
    fn release_id(&mut self, id: usize) -> Result<()> {
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

    pub fn add_item(
        &mut self,
        description: String,
        deadline: Option<DateTime<Local>>,
        parent_id: Option<usize>,
    ) -> Result<&TodoItem> {
        let list = if let Some(p) = parent_id {
            let parent = self
                .items
                .iter()
                .position(|item| item.id == p)
                .ok_or_else(|| anyhow::anyhow!("Parent index {} not found", p))?;

            self.items[parent]
                .sub_list
                .get_or_insert_with(|| TodoList::new())
        } else {
            self
        };
        let id = list.id_pool.acquire_id();
        let time = match deadline {
            None => None,
            Some(deadline) => Some(deadline.to_string()),
        };
        let item = TodoItem {
            id: id,
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

    pub fn list_items(&self) -> Vec<&TodoItem> {
        if SHOW_COMPLETE.load(Ordering::SeqCst) {
            self.items.iter().collect()
        } else {
            self.items.iter().filter(|item| !item.completed).collect()
        }
    }

    pub fn complete_item(&mut self, id: usize) -> Result<&TodoItem> {
        let item = self
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or(anyhow::anyhow!("Item with id {} not found", id))?;

        item.completed = true;
        Ok(item)
    }

    pub fn remove_item(&mut self, id: usize) -> Result<TodoItem> {
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

impl Default for TodoItem {
    fn default() -> Self {
        Self {
            id: 0,
            description: String::new(),
            completed: false,
            deadline: None,
            sub_list: None,
        }
    }
}

impl TodoItem {
    // TODO: 实现Display与Format
    pub fn display(&self, deep: usize) {
        let status = if self.completed { "✓" } else { " " };
        println!(
            "{}[{}] #{}: {}{} {}",
            "  ".repeat(deep),
            status,
            self.id,
            self.description,
            match &self.sub_list {
                Some(list) => format!("({})", list.todo_len()),
                None => String::new(),
            },
            match &self.deadline {
                Some(time) => format!("| deadline: {}", time),
                None => String::new(),
            }
        );
        if let Some(sub_list) = &self.sub_list {
            let items = sub_list.list_items();
            for item in items {
                item.display(deep + 1);
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
