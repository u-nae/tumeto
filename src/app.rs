use std::{io, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Priority {
    #[default]
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn cycle(&self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Low,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub title: String,
    pub completed: bool,

    #[serde(default)]
    pub notes: String,

    #[serde(default)]
    pub priority: Priority,
}

impl TodoItem {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            completed: false,
            notes: String::new(),
            priority: Priority::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Normal,
    Input,
    Help,
    EditingNotes,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FocusedPane {
    #[default]
    List,
    Notes,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct App {
    pub todos: Vec<TodoItem>,

    pub selected: usize,

    #[serde(skip)]
    pub mode: AppMode,

    #[serde(skip)]
    pub input_buffer: String,

    #[serde(skip)]
    pub should_quit: bool,

    #[serde(skip)]
    pub dirty: bool,

    #[serde(skip)]
    pub editing_index: Option<usize>,

    #[serde(skip)]
    pub last_deleted: Option<(usize, TodoItem)>,

    #[serde(skip)]
    pub focused_pane: FocusedPane,

    #[serde(skip)]
    pub notes_buffer: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            todos: Vec::new(),
            selected: 0,
            mode: AppMode::Normal,
            input_buffer: String::new(),
            should_quit: false,
            dirty: false,
            editing_index: None,
            last_deleted: None,
            focused_pane: FocusedPane::List,
            notes_buffer: String::new(),
        }
    }
}

impl App {
    pub fn load() -> Self {
        let path = Self::data_file_path();

        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let content = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        std::fs::write(Self::data_file_path(), content)
    }

    fn data_file_path() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".tudo_data.json")
    }

    pub fn toggle_selected(&mut self) {
        if let Some(item) = self.todos.get_mut(self.selected) {
            item.completed = !item.completed;
            self.dirty = true;
        }
    }

    pub fn delete_selected(&mut self) {
        if self.todos.is_empty() {
            return;
        }

        let removed = self.todos.remove(self.selected);
        self.last_deleted = Some((self.selected, removed));

        if self.todos.is_empty() {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(self.todos.len() - 1);
        }
        self.dirty = true;
    }

    pub fn undo(&mut self) {
        if let Some((idx, item)) = self.last_deleted.take() {
            let insert_at = idx.min(self.todos.len());

            self.todos.insert(insert_at, item);
            self.selected = insert_at;
            self.dirty = true;
        }
    }

    pub fn toggle_help(&mut self) {
        self.mode = match self.mode {
            AppMode::Help => AppMode::Normal,

            _ => AppMode::Help,
        };
    }

    pub fn toggle_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::List => FocusedPane::Notes,
            FocusedPane::Notes => FocusedPane::List,
        };
    }

    pub fn move_down(&mut self) {
        if !self.todos.is_empty() {
            self.selected = (self.selected + 1).min(self.todos.len() - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn enter_input_mode(&mut self) {
        self.mode = AppMode::Input;
        self.input_buffer.clear();
        self.editing_index = None;
    }

    pub fn enter_edit_mode(&mut self) {
        if let Some(item) = self.todos.get(self.selected) {
            self.input_buffer = item.title.clone();
            self.editing_index = Some(self.selected);
            self.mode = AppMode::Input;
        }
    }

    pub fn cancel_input(&mut self) {
        self.mode = AppMode::Normal;
        self.input_buffer.clear();
        self.editing_index = None;
    }

    pub fn commit_input(&mut self) {
        let title = self.input_buffer.trim().to_owned();

        if !title.is_empty() {
            match self.editing_index {
                Some(idx) => {
                    if let Some(item) = self.todos.get_mut(idx)
                        && item.title != title {
                            item.title = title;
                            self.dirty = true;
                        }
                }
                None => {
                    self.todos.push(TodoItem::new(title));
                    self.selected = self.todos.len() - 1;
                    self.dirty = true;
                }
            }
        }

        self.input_buffer.clear();
        self.editing_index = None;
        self.mode = AppMode::Normal;
    }

    pub fn cycle_priority(&mut self) {
        if let Some(item) = self.todos.get_mut(self.selected) {
            item.priority = item.priority.cycle();
            self.dirty = true;
        }
    }

    pub fn enter_notes_mode(&mut self) {
        if self.todos.is_empty() {
            return;
        }

        self.notes_buffer = self.todos[self.selected].notes.clone();

        self.focused_pane = FocusedPane::Notes;
        self.mode = AppMode::EditingNotes;
    }

    pub fn commit_notes(&mut self) {
        if let Some(item) = self.todos.get_mut(self.selected) {
            if item.notes != self.notes_buffer {
                item.notes = std::mem::take(&mut self.notes_buffer);
                self.dirty = true;
            } else {
                self.notes_buffer.clear();
            }
        }
        self.mode = AppMode::Normal;
    }

    pub fn cancel_notes(&mut self) {
        self.notes_buffer.clear();
        self.mode = AppMode::Normal;
    }
}
