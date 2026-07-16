use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::app::{App, AppMode, FocusedPane, Priority, TodoItem};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let outer_block = Block::default()
        .title(" tudo ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    match app.mode {
        AppMode::Normal => render_normal(frame, app, inner_area),
        AppMode::Input => render_input(frame, app, inner_area),
        AppMode::Search => render_search(frame, app, inner_area),
        AppMode::Help => {
            render_normal(frame, app, inner_area);
            render_help_overlay(frame, area);
        }
        AppMode::EditingNotes => render_editing_notes(frame, app, inner_area),
    }
}

fn render_normal(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    render_content(frame, app, chunks[0]);
    render_footer(frame, chunks[1], FooterMode::Normal);
}

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_list_pane(frame, app, panes[0]);
    render_notes_pane(frame, app, panes[1]);
}

fn render_list_pane(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focused_pane == FocusedPane::List {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if app.search_query.is_empty() {
        " 할 일 ".to_string()
    } else {
        format!(" 할 일 (검색: {}) ", app.search_query)
    };

    let panel = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    render_list(frame, app, inner);
}

fn render_notes_pane(frame: &mut Frame, app: &App, area: Rect) {
    let is_editing = app.mode == AppMode::EditingNotes;

    let (title, border_color) = if is_editing {
        (" 메모 (편집 중) ", Color::Yellow)
    } else if app.focused_pane == FocusedPane::Notes {
        (" 메모 ", Color::White)
    } else {
        (" 메모 ", Color::DarkGray)
    };

    let panel = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    render_notes_content(frame, app, inner);
}

fn render_notes_content(frame: &mut Frame, app: &App, area: Rect) {
    let is_editing = app.mode == AppMode::EditingNotes;

    let content = if is_editing {
        app.notes_buffer.as_str()
    } else {
        app.current_index()
            .map(|index| app.todos[index].notes.as_str())
            .unwrap_or("")
    };

    if content.is_empty() && !is_editing {
        let placeholder = Paragraph::new("[ m ] 키로 메모를 작성하세요.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(placeholder, area);
    } else {
        let style = if is_editing {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let safe_width = area.width.max(1);
        let wrapped_lines = wrap_text(content, safe_width);

        let lines_for_widget: Vec<Line> = wrapped_lines
            .iter()
            .map(|l| Line::from(Span::styled(l.clone(), style)))
            .collect();
        let notes_widget = Paragraph::new(lines_for_widget);
        frame.render_widget(notes_widget, area);

        if is_editing {
            let last_line = wrapped_lines.last().unwrap_or(&String::new()).clone();

            let cursor_x = area.x + last_line.width() as u16;

            let cursor_y = area.y + wrapped_lines.len().saturating_sub(1) as u16;

            frame.set_cursor_position((
                cursor_x.min(area.right().saturating_sub(1)),
                cursor_y.min(area.bottom().saturating_sub(1)),
            ));
        }
    }
}

fn wrap_text(text: &str, panel_width: u16) -> Vec<String> {
    let mut lines = Vec::new();

    for logical_line in text.split('\n') {
        let mut current_line = String::new();
        let mut current_width = 0;

        for c in logical_line.chars() {
            let char_width = c.width().unwrap_or(0) as u16;

            if current_width + char_width > panel_width {
                lines.push(current_line);
                current_line = String::new();
                current_width = 0;
            }
            current_line.push(c);
            current_width += char_width;
        }
        lines.push(current_line);
    }
    lines
}

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    render_content(frame, app, chunks[0]);
    render_input_box(frame, app, chunks[1]);
    render_footer(frame, chunks[2], FooterMode::Input);
}

fn render_editing_notes(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    render_content(frame, app, chunks[0]);
    render_footer(frame, chunks[1], FooterMode::EditingNotes);
}

fn render_list(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.visible_indices();

    if visible.is_empty() {
        let message = if app.search_query.is_empty() {
            "아직 할 일이 없습니다. [ a ] 를 눌러 추가하세요."
        } else {
            "검색 결과가 없습니다."
        };
        let empty_msg = Paragraph::new(message)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(empty_msg, area);
        return;
    }

    let items: Vec<ListItem> = visible
        .iter()
        .map(|&index| build_list_item(&app.todos[index]))
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn build_list_item(todo: &TodoItem) -> ListItem<'_> {
    if todo.completed {
        return ListItem::new(Line::from(vec![
            Span::styled(" [x] ", Style::default().fg(Color::Green)),
            Span::styled(
                todo.title.clone(),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT),
            ),
        ]));
    }

    let title_color = match todo.priority {
        Priority::High => Color::Red,
        Priority::Medium => Color::Yellow,
        Priority::Low => Color::White,
    };

    let badge: Option<Span> = match todo.priority {
        Priority::High => Some(Span::styled(
            "  !!",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Priority::Medium => Some(Span::styled("  ! ", Style::default().fg(Color::Yellow))),
        Priority::Low => None,
    };

    let mut spans = vec![
        Span::styled(" [ ] ", Style::default().fg(Color::White)),
        Span::styled(todo.title.clone(), Style::default().fg(title_color)),
    ];

    spans.extend(badge);

    ListItem::new(Line::from(spans))
}

fn render_input_box(frame: &mut Frame, app: &App, area: Rect) {
    let (title, color) = if app.editing_index.is_some() {
        (" 항목 편집 ", Color::Green)
    } else {
        (" 새 할 일 입력 ", Color::Yellow)
    };

    let input_box = Paragraph::new(app.input_buffer.as_str())
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color)),
        )
        .style(Style::default().fg(color));

    frame.render_widget(input_box, area);

    let cursor_x =
        (area.x + 1 + app.input_buffer.width() as u16).min(area.right().saturating_sub(2));
    let cursor_y = area.y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn render_search(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    render_content(frame, app, chunks[0]);
    render_search_box(frame, app, chunks[1]);
    render_footer(frame, chunks[2], FooterMode::Search);
}

fn render_search_box(frame: &mut Frame, app: &App, area: Rect) {
    let search_box = Paragraph::new(app.search_query.as_str())
        .block(
            Block::default()
                .title(" 검색 ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .style(Style::default().fg(Color::Magenta));

    frame.render_widget(search_box, area);

    let cursor_x =
        (area.x + 1 + app.search_query.width() as u16).min(area.right().saturating_sub(2));
    frame.set_cursor_position((cursor_x, area.y + 1));
}

fn render_footer(frame: &mut Frame, area: Rect, mode: FooterMode) {
    let text = match mode {
        FooterMode::Normal => {
            "[ j/k ] 이동  [ a/e/d ] 추가·편집·삭제  [ p ] 우선순위  [ u ] 되돌리기  [ ? ] 도움말  [ q ] 종료"
        }
        FooterMode::Input => "[ Enter ] 저장  [ Esc ] 취소",
        FooterMode::EditingNotes => {
            "[ Ctrl+S ] 메모 저장  [ Esc ] 취소  [ Enter ] 줄 바꿈  [ Backspace ] 삭제"
        }
        FooterMode::Search => "[ Enter ] 검색 확정  [ Esc ] 검색 해제  [ 문자 ] 실시간 필터",
    };

    let footer = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(56, 80, area);

    frame.render_widget(Clear, popup_area);

    let key = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let desc = Style::default().fg(Color::White);
    let head = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(Span::styled(" Normal 모드", head)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j / ↓      ", key),
            Span::styled("아래로 이동", desc),
        ]),
        Line::from(vec![
            Span::styled("  k / ↑      ", key),
            Span::styled("위로 이동", desc),
        ]),
        Line::from(vec![
            Span::styled("  Space      ", key),
            Span::styled("완료 토글", desc),
        ]),
        Line::from(vec![
            Span::styled("  a / i      ", key),
            Span::styled("새 항목 추가", desc),
        ]),
        Line::from(vec![
            Span::styled("  e          ", key),
            Span::styled("항목 편집", desc),
        ]),
        Line::from(vec![
            Span::styled("  d / x      ", key),
            Span::styled("항목 삭제", desc),
        ]),
        Line::from(vec![
            Span::styled("  u          ", key),
            Span::styled("삭제 되돌리기 (Undo)", desc),
        ]),
        Line::from(vec![
            Span::styled("  Tab        ", key),
            Span::styled("패널 포커스 전환", desc),
        ]),
        Line::from(vec![
            Span::styled("  m          ", key),
            Span::styled("메모 편집", desc),
        ]),
        Line::from(vec![
            Span::styled("  p          ", key),
            Span::styled("우선순위 순환 (Low → Medium → High)", desc),
        ]),
        Line::from(vec![
            Span::styled("  /          ", key),
            Span::styled("제목 검색 / 필터", desc),
        ]),
        Line::from(vec![
            Span::styled("  ?          ", key),
            Span::styled("도움말 닫기", desc),
        ]),
        Line::from(vec![
            Span::styled("  q          ", key),
            Span::styled("종료", desc),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C     ", key),
            Span::styled("즉시 종료", desc),
        ]),
        Line::from(""),
        Line::from(Span::styled(" 입력 / 편집 모드", head)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter      ", key),
            Span::styled("저장", desc),
        ]),
        Line::from(vec![
            Span::styled("  Esc        ", key),
            Span::styled("취소", desc),
        ]),
        Line::from(vec![
            Span::styled("  Backspace  ", key),
            Span::styled("마지막 문자 삭제", desc),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(" 우선순위 색상", head)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Red        ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("High", desc),
        ]),
        Line::from(vec![
            Span::styled(
                "  Yellow     ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Medium", desc),
        ]),
        Line::from(vec![
            Span::styled("  White      ", Style::default().fg(Color::White)),
            Span::styled("Low (기본값)", desc),
        ]),
        Line::from(""),
        Line::from(Span::styled(" 검색 모드", head)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  문자 입력  ", key),
            Span::styled("실시간 필터 (대소문자 무시)", desc),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", key),
            Span::styled("검색 확정 (필터 유지)", desc),
        ]),
        Line::from(vec![
            Span::styled("  Esc        ", key),
            Span::styled("검색 해제 (전체 표시)", desc),
        ]),
        Line::from(""),
        Line::from(Span::styled(" 메모 편집 모드", head)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Ctrl+S     ", key),
            Span::styled("메모 저장", desc),
        ]),
        Line::from(vec![
            Span::styled("  Esc        ", key),
            Span::styled("편집 취소", desc),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", key),
            Span::styled("줄 바꿈 삽입", desc),
        ]),
        Line::from(vec![
            Span::styled("  Backspace  ", key),
            Span::styled("마지막 문자 삭제", desc),
        ]),
        Line::from(""),
        Line::from(Span::styled("  아무 키나 눌러 닫기", dim)),
    ];

    let popup = Paragraph::new(lines).block(
        Block::default()
            .title(" 도움말 [ ? ] ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(popup, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

enum FooterMode {
    Normal,
    Input,
    EditingNotes,
    Search,
}
