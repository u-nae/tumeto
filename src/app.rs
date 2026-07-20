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
pub struct SubTask {
    pub title: String,
    pub completed: bool,
}

impl SubTask {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            completed: false,
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

    #[serde(default)]
    pub subtasks: Vec<SubTask>,

    #[serde(default)]
    pub collapsed: bool,
}

impl TodoItem {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            completed: false,
            notes: String::new(),
            priority: Priority::default(),
            subtasks: Vec::new(),
            collapsed: false,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowRef {
    Todo(usize),
    Sub(usize, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputTarget {
    #[default]
    NewTodo,
    NewSubtask(usize),
    EditTodo(usize),
    EditSubtask(usize, usize),
}

#[derive(Debug, Clone)]
pub enum LastDeleted {
    Todo {
        group: usize,
        index: usize,
        item: TodoItem,
    },
    Sub {
        group: usize,
        todo: usize,
        index: usize,
        item: SubTask,
    },
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
    GroupInput,
    GroupDeleteConfirm,
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
    pub input_target: InputTarget,

    #[serde(skip)]
    pub last_deleted: Option<LastDeleted>,

    #[serde(skip)]
    pub notes_buffer: String,

    #[serde(skip)]
    pub search_query: String,

    #[serde(skip)]
    pub category_cursor: usize,

    #[serde(skip)]
    pub group_editing: Option<usize>,
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
            input_target: InputTarget::NewTodo,
            last_deleted: None,
            notes_buffer: String::new(),
            search_query: String::new(),
            category_cursor: 0,
            group_editing: None,
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
        PathBuf::from(home).join(".tumeto_data.json")
    }

    pub fn toggle_selected(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let Some(group) = self.groups.get_mut(self.selected_group) else {
            return;
        };
        match row {
            RowRef::Todo(i) => {
                let new_state = !group.todos[i].completed;
                group.todos[i].completed = new_state;
                for sub in &mut group.todos[i].subtasks {
                    sub.completed = new_state;
                }
            }
            RowRef::Sub(i, j) => {
                group.todos[i].subtasks[j].completed = !group.todos[i].subtasks[j].completed;
            }
        }
        self.dirty = true;
    }

    pub fn delete_selected(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let group_idx = self.selected_group;
        let Some(group) = self.groups.get_mut(group_idx) else {
            return;
        };
        match row {
            RowRef::Todo(i) => {
                let item = group.todos.remove(i);
                self.last_deleted = Some(LastDeleted::Todo {
                    group: group_idx,
                    index: i,
                    item,
                });
            }
            RowRef::Sub(i, j) => {
                let item = group.todos[i].subtasks.remove(j);
                self.last_deleted = Some(LastDeleted::Sub {
                    group: group_idx,
                    todo: i,
                    index: j,
                    item,
                });
            }
        }
        self.dirty = true;
        self.clamp_selection();
    }

    pub fn undo(&mut self) {
        let Some(deleted) = self.last_deleted.take() else {
            return;
        };
        match deleted {
            LastDeleted::Todo { group, index, item } => {
                let Some(g) = self.groups.get_mut(group) else {
                    return;
                };
                let at = index.min(g.todos.len());
                g.todos.insert(at, item);
                self.selected_group = group;
                self.dirty = true;
                self.select_row(RowRef::Todo(at));
            }
            LastDeleted::Sub {
                group,
                todo,
                index,
                item,
            } => {
                let Some(g) = self.groups.get_mut(group) else {
                    return;
                };
                let Some(t) = g.todos.get_mut(todo) else {
                    return;
                };
                let at = index.min(t.subtasks.len());
                t.subtasks.insert(at, item);
                t.collapsed = false;
                self.selected_group = group;
                self.dirty = true;
                self.select_row(RowRef::Sub(todo, at));
            }
        }
    }

    pub fn toggle_help(&mut self) {
        self.mode = match self.mode {
            AppMode::Help => AppMode::Normal,

            _ => AppMode::Help,
        };
    }

    pub fn move_down(&mut self) {
        let count = self.visible_rows().len();
        if count > 0 {
            self.selected = (self.selected + 1).min(count - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn enter_edit_mode(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let Some(group) = self.groups.get(self.selected_group) else {
            return;
        };
        let (title, target) = match row {
            RowRef::Todo(i) => (group.todos[i].title.clone(), InputTarget::EditTodo(i)),
            RowRef::Sub(i, j) => (
                group.todos[i].subtasks[j].title.clone(),
                InputTarget::EditSubtask(i, j),
            ),
        };
        self.input_buffer = title;
        self.input_target = target;
        self.mode = AppMode::Input;
    }

    pub fn enter_input_mode(&mut self) {
        self.mode = AppMode::Input;
        self.input_buffer.clear();
        self.input_target = InputTarget::NewTodo;
    }

    pub fn cancel_input(&mut self) {
        self.mode = AppMode::Normal;
        self.input_buffer.clear();
        self.input_target = InputTarget::NewTodo;
    }

    pub fn commit_input(&mut self) {
        let title = self.input_buffer.trim().to_owned();
        let mut to_select: Option<RowRef> = None;

        if !title.is_empty() {
            match self.input_target {
                InputTarget::NewTodo => {
                    if let Some(group) = self.groups.get_mut(self.selected_group) {
                        group.todos.push(TodoItem::new(title));
                        to_select = Some(RowRef::Todo(group.todos.len() - 1));
                        self.dirty = true;
                    }
                }
                InputTarget::NewSubtask(parent) => {
                    if let Some(group) = self.groups.get_mut(self.selected_group)
                        && let Some(todo) = group.todos.get_mut(parent)
                    {
                        todo.subtasks.push(SubTask::new(title));
                        todo.collapsed = false;
                        to_select = Some(RowRef::Sub(parent, todo.subtasks.len() - 1));
                        self.dirty = true;
                    }
                }
                InputTarget::EditTodo(i) => {
                    if let Some(group) = self.groups.get_mut(self.selected_group)
                        && let Some(todo) = group.todos.get_mut(i)
                        && todo.title != title
                    {
                        todo.title = title;
                        self.dirty = true;
                    }
                }
                InputTarget::EditSubtask(i, j) => {
                    if let Some(group) = self.groups.get_mut(self.selected_group)
                        && let Some(todo) = group.todos.get_mut(i)
                        && let Some(sub) = todo.subtasks.get_mut(j)
                        && sub.title != title
                    {
                        sub.title = title;
                        self.dirty = true;
                    }
                }
            }
        }

        self.input_buffer.clear();
        self.input_target = InputTarget::NewTodo;
        self.mode = AppMode::Normal;
        if let Some(row) = to_select {
            self.select_row(row);
        }
        self.clamp_selection();
    }

    pub fn enter_subtask_input(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let parent = match row {
            RowRef::Todo(i) => i,
            RowRef::Sub(i, _) => i,
        };
        self.input_buffer.clear();
        self.input_target = InputTarget::NewSubtask(parent);
        self.mode = AppMode::Input;
    }

    pub fn cycle_priority(&mut self) {
        let Some(RowRef::Todo(i)) = self.current_row() else {
            return;
        };
        if let Some(group) = self.groups.get_mut(self.selected_group) {
            group.todos[i].priority = group.todos[i].priority.cycle();
            self.dirty = true;
        }
    }

    pub fn enter_notes_mode(&mut self) {
        let Some(RowRef::Todo(i)) = self.current_row() else {
            return;
        };
        if let Some(group) = self.groups.get(self.selected_group) {
            self.notes_buffer = group.todos[i].notes.clone();
            self.mode = AppMode::EditingNotes;
        }
    }

    pub fn commit_notes(&mut self) {
        let Some(RowRef::Todo(i)) = self.current_row() else {
            self.mode = AppMode::Normal;
            return;
        };
        if let Some(group) = self.groups.get_mut(self.selected_group) {
            if group.todos[i].notes != self.notes_buffer {
                group.todos[i].notes = std::mem::take(&mut self.notes_buffer);
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

    pub fn visible_rows(&self) -> Vec<RowRef> {
        let Some(group) = self.groups.get(self.selected_group) else {
            return Vec::new();
        };
        let query = self.search_query.to_lowercase();
        let mut rows = Vec::new();
        for (i, todo) in group.todos.iter().enumerate() {
            if !query.is_empty() && !todo.title.to_lowercase().contains(&query) {
                continue;
            }
            rows.push(RowRef::Todo(i));
            if !todo.collapsed {
                for j in 0..todo.subtasks.len() {
                    rows.push(RowRef::Sub(i, j));
                }
            }
        }
        rows
    }

    pub fn current_row(&self) -> Option<RowRef> {
        self.visible_rows().get(self.selected).copied()
    }

    pub fn current_todos(&self) -> &[TodoItem] {
        self.groups
            .get(self.selected_group)
            .map(|g| g.todos.as_slice())
            .unwrap_or(&[])
    }

    fn clamp_selection(&mut self) {
        let count = self.visible_rows().len();
        self.selected = if count == 0 {
            0
        } else {
            self.selected.min(count - 1)
        };
    }

    fn select_row(&mut self, target: RowRef) {
        if let Some(pos) = self.visible_rows().iter().position(|&r| r == target) {
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

    pub fn toggle_collapse(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let parent = match row {
            RowRef::Todo(i) => i,
            RowRef::Sub(i, _) => i,
        };
        if let Some(group) = self.groups.get_mut(self.selected_group) {
            let Some(todo) = group.todos.get_mut(parent) else {
                return;
            };
            if todo.subtasks.is_empty() {
                return;
            }
            todo.collapsed = !todo.collapsed;
        }
        self.select_row(RowRef::Todo(parent));
        self.clamp_selection();
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

    pub fn enter_group_add(&mut self) {
        self.group_editing = None;
        self.input_buffer.clear();
        self.mode = AppMode::GroupInput;
    }

    pub fn enter_group_rename(&mut self) {
        if let Some(group) = self.groups.get(self.category_cursor) {
            self.input_buffer = group.name.clone();
            self.group_editing = Some(self.category_cursor);
            self.mode = AppMode::GroupInput;
        }
    }

    pub fn commit_group_input(&mut self) {
        let name = self.input_buffer.trim().to_owned();
        if !name.is_empty() {
            match self.group_editing {
                Some(i) => {
                    if let Some(group) = self.groups.get_mut(i)
                        && group.name != name
                    {
                        group.name = name;
                        self.dirty = true;
                    }
                }
                None => {
                    self.groups.push(Group::new(name));
                    self.category_cursor = self.groups.len() - 1;
                    self.dirty = true;
                }
            }
        }
        self.input_buffer.clear();
        self.group_editing = None;
        self.mode = AppMode::CategoryPopup;
    }

    pub fn cancel_group_input(&mut self) {
        self.input_buffer.clear();
        self.group_editing = None;
        self.mode = AppMode::CategoryPopup;
    }

    pub fn enter_group_delete_confirm(&mut self) {
        if self.groups.len() <= 1 {
            return;
        }
        self.mode = AppMode::GroupDeleteConfirm;
    }

    pub fn confirm_group_delete(&mut self) {
        if self.groups.len() <= 1 {
            self.mode = AppMode::CategoryPopup;
            return;
        }
        let target = self.category_cursor.min(self.groups.len() - 1);
        self.groups.remove(target);

        if self.selected_group == target {
            self.selected_group = self.selected_group.min(self.groups.len() - 1);
            self.reset_group_view();
        } else if self.selected_group > target {
            self.selected_group -= 1;
        }

        self.category_cursor = self.category_cursor.min(self.groups.len() - 1);
        self.last_deleted = None;
        self.dirty = true;
        self.mode = AppMode::CategoryPopup;
    }

    pub fn cancel_group_delete(&mut self) {
        self.mode = AppMode::CategoryPopup;
    }
}
