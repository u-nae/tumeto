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
    Search,
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

    #[serde(skip)]
    pub search_query: String,
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
            search_query: String::new(),
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
        if let Some(index) = self.current_index() {
            self.todos[index].completed = !self.todos[index].completed;
            self.dirty = true;
        }
    }

    pub fn delete_selected(&mut self) {
        let Some(index) = self.current_index() else {
            return;
        };
        let removed = self.todos.remove(index);
        self.last_deleted = Some((index, removed));
        self.dirty = true;
        self.clamp_selection();
    }

    pub fn undo(&mut self) {
        if let Some((idx, item)) = self.last_deleted.take() {
            let insert_at = idx.min(self.todos.len());
            self.todos.insert(insert_at, item);
            self.dirty = true;
            self.select_real_index(insert_at);
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
        let count = self.visible_indices().len();
        if count > 0 {
            self.selected = (self.selected + 1).min(count - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn enter_edit_mode(&mut self) {
        if let Some(index) = self.current_index() {
            self.input_buffer = self.todos[index].title.clone();
            self.editing_index = Some(index);
            self.mode = AppMode::Input;
        }
    }

    pub fn enter_input_mode(&mut self) {
        self.mode = AppMode::Input;
        self.input_buffer.clear();
        self.editing_index = None;
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
                Some(index) => {
                    if let Some(item) = self.todos.get_mut(index)
                        && item.title != title
                    {
                        item.title = title;
                        self.dirty = true;
                    }
                }
                None => {
                    self.todos.push(TodoItem::new(title));
                    self.dirty = true;
                    self.select_real_index(self.todos.len() - 1);
                }
            }
        }

        self.input_buffer.clear();
        self.editing_index = None;
        self.mode = AppMode::Normal;
        self.clamp_selection();
    }

    pub fn cycle_priority(&mut self) {
        if let Some(index) = self.current_index() {
            self.todos[index].priority = self.todos[index].priority.cycle();
            self.dirty = true;
        }
    }

    pub fn enter_notes_mode(&mut self) {
        let Some(index) = self.current_index() else {
            return;
        };
        self.notes_buffer = self.todos[index].notes.clone();
        self.focused_pane = FocusedPane::Notes;
        self.mode = AppMode::EditingNotes;
    }

    pub fn commit_notes(&mut self) {
        if let Some(index) = self.current_index() {
            if self.todos[index].notes != self.notes_buffer {
                self.todos[index].notes = std::mem::take(&mut self.notes_buffer);
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

    pub fn visible_indices(&self) -> Vec<usize> {
        if self.search_query.is_empty() {
            return (0..self.todos.len()).collect();
        }
        let query = self.search_query.to_lowercase();
        self.todos
            .iter()
            .enumerate()
            .filter(|(_, item)| item.title.to_lowercase().contains(&query))
            .map(|(index, _)| index)
            .collect()
    }

    pub fn current_index(&self) -> Option<usize> {
        self.visible_indices().get(self.selected).copied()
    }

    fn clamp_selection(&mut self) {
        let count = self.visible_indices().len();
        self.selected = if count == 0 {
            0
        } else {
            self.selected.min(count - 1)
        };
    }

    fn select_real_index(&mut self, real_index: usize) {
        if let Some(pos) = self.visible_indices().iter().position(|&i| i == real_index) {
            self.selected = pos;
        }
    }

    pub fn enter_search_mode(&mut self) {
        self.mode = AppMode::Search;
    }

    pub fn push_search(&mut self, c: char) {
        self.search_query.push(c);
        self.clamp_selection();
    }

    pub fn backspace_search(&mut self) {
        self.search_query.pop();
        self.clamp_selection();
    }

    pub fn confirm_search(&mut self) {
        self.mode = AppMode::Normal;
        self.clamp_selection();
    }

    pub fn cancel_search(&mut self) {
        self.search_query.clear();
        self.mode = AppMode::Normal;
        self.clamp_selection();
    }
}
