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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,

    #[serde(default)]
    pub todos: Vec<TodoItem>,
}

impl Group {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            todos: Vec::new(),
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
    CategoryPopup,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct App {
    pub groups: Vec<Group>,

    #[serde(default)]
    pub selected_group: usize,

    #[serde(default)]
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
    pub last_deleted: Option<(usize, usize, TodoItem)>,

    #[serde(skip)]
    pub notes_buffer: String,

    #[serde(skip)]
    pub search_query: String,

    #[serde(skip)]
    pub category_cursor: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            groups: vec![Group::new("기본")],
            selected_group: 0,
            selected: 0,
            mode: AppMode::Normal,
            input_buffer: String::new(),
            should_quit: false,
            dirty: false,
            editing_index: None,
            last_deleted: None,
            notes_buffer: String::new(),
            search_query: String::new(),
            category_cursor: 0,
        }
    }
}

#[derive(Deserialize)]
struct LegacyApp {
    #[serde(default)]
    todos: Vec<TodoItem>,
    #[serde(default)]
    selected: usize,
}

impl App {
    pub fn load() -> Self {
        let path = Self::data_file_path();

        let Ok(content) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
            return Self::default();
        };

        if value.get("groups").is_some() {
            serde_json::from_value(value).unwrap_or_default()
        } else {
            serde_json::from_value::<LegacyApp>(value)
                .map(Self::from_legacy)
                .unwrap_or_default()
        }
    }

    fn from_legacy(legacy: LegacyApp) -> Self {
        Self {
            groups: vec![Group {
                name: "기본".to_string(),
                todos: legacy.todos,
            }],
            selected_group: 0,
            selected: legacy.selected,
            ..Self::default()
        }
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
        let Some(index) = self.current_index() else {
            return;
        };
        if let Some(group) = self.groups.get_mut(self.selected_group) {
            group.todos[index].completed = !group.todos[index].completed;
            self.dirty = true;
        }
    }

    pub fn delete_selected(&mut self) {
        let Some(index) = self.current_index() else {
            return;
        };
        let group_idx = self.selected_group;
        if let Some(group) = self.groups.get_mut(group_idx) {
            let removed = group.todos.remove(index);
            self.last_deleted = Some((group_idx, index, removed));
            self.dirty = true;
        }
        self.clamp_selection();
    }

    pub fn undo(&mut self) {
        let Some((group_idx, todo_idx, item)) = self.last_deleted.take() else {
            return;
        };
        let Some(group) = self.groups.get_mut(group_idx) else {
            return;
        };
        let insert_at = todo_idx.min(group.todos.len());
        group.todos.insert(insert_at, item);
        self.dirty = true;
        self.selected_group = group_idx;
        self.select_real_index(insert_at);
    }

    pub fn toggle_help(&mut self) {
        self.mode = match self.mode {
            AppMode::Help => AppMode::Normal,

            _ => AppMode::Help,
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
        let Some(index) = self.current_index() else {
            return;
        };
        if let Some(group) = self.groups.get(self.selected_group) {
            self.input_buffer = group.todos[index].title.clone();
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
                    if let Some(group) = self.groups.get_mut(self.selected_group)
                        && let Some(item) = group.todos.get_mut(index)
                        && item.title != title
                    {
                        item.title = title;
                        self.dirty = true;
                    }
                }
                None => {
                    if let Some(group) = self.groups.get_mut(self.selected_group) {
                        group.todos.push(TodoItem::new(title));
                        self.dirty = true;
                    }
                    let last = self.current_todos().len().saturating_sub(1);
                    self.select_real_index(last);
                }
            }
        }

        self.input_buffer.clear();
        self.editing_index = None;
        self.mode = AppMode::Normal;
        self.clamp_selection();
    }

    pub fn cycle_priority(&mut self) {
        let Some(index) = self.current_index() else {
            return;
        };
        if let Some(group) = self.groups.get_mut(self.selected_group) {
            group.todos[index].priority = group.todos[index].priority.cycle();
            self.dirty = true;
        }
    }

    pub fn enter_notes_mode(&mut self) {
        let Some(index) = self.current_index() else {
            return;
        };
        if let Some(group) = self.groups.get(self.selected_group) {
            self.notes_buffer = group.todos[index].notes.clone();
            self.mode = AppMode::EditingNotes;
        }
    }

    pub fn commit_notes(&mut self) {
        let Some(index) = self.current_index() else {
            self.mode = AppMode::Normal;
            return;
        };
        if let Some(group) = self.groups.get_mut(self.selected_group) {
            if group.todos[index].notes != self.notes_buffer {
                group.todos[index].notes = std::mem::take(&mut self.notes_buffer);
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
        let Some(group) = self.groups.get(self.selected_group) else {
            return Vec::new();
        };
        if self.search_query.is_empty() {
            return (0..group.todos.len()).collect();
        }
        let query = self.search_query.to_lowercase();
        group
            .todos
            .iter()
            .enumerate()
            .filter(|(_, item)| item.title.to_lowercase().contains(&query))
            .map(|(index, _)| index)
            .collect()
    }

    pub fn current_index(&self) -> Option<usize> {
        self.visible_indices().get(self.selected).copied()
    }

    pub fn current_todos(&self) -> &[TodoItem] {
        self.groups
            .get(self.selected_group)
            .map(|g| g.todos.as_slice())
            .unwrap_or(&[])
    }

    pub fn current_group_name(&self) -> &str {
        self.groups
            .get(self.selected_group)
            .map(|g| g.name.as_str())
            .unwrap_or("")
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

    pub fn next_group(&mut self) {
        if self.groups.len() <= 1 {
            return;
        }
        self.selected_group = (self.selected_group + 1) % self.groups.len();
        self.reset_group_view();
    }

    pub fn prev_group(&mut self) {
        if self.groups.len() <= 1 {
            return;
        }
        self.selected_group = (self.selected_group + self.groups.len() - 1) % self.groups.len();
        self.reset_group_view();
    }

    pub fn switch_group(&mut self, index: usize) {
        if index < self.groups.len() {
            self.selected_group = index;
            self.reset_group_view();
        }
    }

    fn reset_group_view(&mut self) {
        self.selected = 0;
        self.search_query.clear();
    }

    pub fn enter_category_popup(&mut self) {
        self.category_cursor = self.selected_group;
        self.mode = AppMode::CategoryPopup;
    }

    pub fn category_cursor_up(&mut self) {
        self.category_cursor = self.category_cursor.saturating_sub(1);
    }

    pub fn category_cursor_down(&mut self) {
        if self.category_cursor + 1 < self.groups.len() {
            self.category_cursor += 1;
        }
    }

    pub fn confirm_category_popup(&mut self) {
        self.switch_group(self.category_cursor);
        self.mode = AppMode::Normal;
    }

    pub fn cancel_category_popup(&mut self) {
        self.mode = AppMode::Normal;
    }
}
