use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::io;

use crate::app::{App, AppMode};

pub fn handle_events(app: &mut App) -> io::Result<()> {
    if event::poll(std::time::Duration::from_millis(16))?
        && let Event::Key(key) = event::read()?
    {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
            app.should_quit = true;
            return Ok(());
        }

        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('s') {
            if app.mode == AppMode::EditingNotes {
                app.commit_notes();
            }
            return Ok(());
        }

        match app.mode {
            AppMode::Normal => handle_normal_mode(app, key.code),
            AppMode::Input => handle_input_mode(app, key.code),
            AppMode::Search => handle_search_mode(app, key.code),
            AppMode::Help => app.toggle_help(),
            AppMode::EditingNotes => handle_editing_notes_mode(app, key.code),
            AppMode::CategoryPopup => handle_category_popup(app, key.code),
            AppMode::GroupInput => handle_group_input(app, key.code),
            AppMode::GroupDeleteConfirm => handle_group_delete_confirm(app, key.code),
        }
    }
    Ok(())
}

fn handle_normal_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,

        KeyCode::Char('j') | KeyCode::Down => app.move_down(),

        KeyCode::Char('k') | KeyCode::Up => app.move_up(),

        KeyCode::Char(' ') => app.cycle_status(),

        KeyCode::Char('a') | KeyCode::Char('i') => app.enter_input_mode(),

        KeyCode::Char('d') | KeyCode::Char('x') => app.delete_selected(),

        KeyCode::Char('e') => app.enter_edit_mode(),

        KeyCode::Char('u') => app.undo(),

        KeyCode::Char('?') => app.toggle_help(),

        KeyCode::Tab | KeyCode::Char('l') => app.next_group(),

        KeyCode::BackTab | KeyCode::Char('h') => app.prev_group(),

        KeyCode::Char('c') => app.enter_category_popup(),

        KeyCode::Char('m') => app.enter_notes_mode(),

        KeyCode::Char('p') => app.cycle_priority(),

        KeyCode::Char('s') => app.enter_subtask_input(),

        KeyCode::Char('z') => app.toggle_collapse(),

        KeyCode::Char('/') => app.enter_search_mode(),

        _ => {}
    }
}

fn handle_input_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.commit_input(),

        KeyCode::Esc => app.cancel_input(),

        KeyCode::Backspace => {
            let _ = app.input_buffer.pop();
        }

        KeyCode::Char(c) => app.input_buffer.push(c),

        _ => {}
    }
}

fn handle_editing_notes_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.cancel_notes(),

        KeyCode::Enter => app.notes_buffer.push('\n'),

        KeyCode::Backspace => {
            let _ = app.notes_buffer.pop();
        }

        KeyCode::Char(c) => app.notes_buffer.push(c),

        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.confirm_search(),
        KeyCode::Esc => app.cancel_search(),
        KeyCode::Backspace => app.backspace_search(),
        KeyCode::Char(c) => app.push_search(c),
        _ => {}
    }
}

fn handle_category_popup(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => app.category_cursor_up(),
        KeyCode::Down | KeyCode::Char('j') => app.category_cursor_down(),
        KeyCode::Char('n') => app.enter_group_add(),
        KeyCode::Char('r') => app.enter_group_rename(),
        KeyCode::Char('d') => app.enter_group_delete_confirm(),
        KeyCode::Enter => app.confirm_category_popup(),
        KeyCode::Esc => app.cancel_category_popup(),
        _ => {}
    }
}

fn handle_group_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.commit_group_input(),
        KeyCode::Esc => app.cancel_group_input(),
        KeyCode::Backspace => {
            let _ = app.input_buffer.pop();
        }
        KeyCode::Char(c) => app.input_buffer.push(c),
        _ => {}
    }
}

fn handle_group_delete_confirm(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_group_delete(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_group_delete(),
        _ => {}
    }
}
