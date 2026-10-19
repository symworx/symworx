// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Key state for the two-pane browser. The event loop performs the actions.

use crossterm::event::{
    KeyCode,
    KeyModifiers,
};

use crate::model::{
    PAGE_SIZE,
    Page,
    Relation,
};

/// What the event loop should do after a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Stay in the loop.
    None,
    /// Leave the terminal.
    Quit,
    /// Reload the relation list.
    ReloadRelations,
    /// Fetch a page of the selected relation.
    LoadPage {
        /// Zero-based row offset.
        offset: u32,
    },
}

/// Which pane receives `j` / `k`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// Left list.
    Relations,
    /// Right grid.
    Rows,
}

/// Browser state. The open database stays outside this struct.
pub struct App {
    /// Connection label (path or redacted host).
    pub label: String,
    /// dbSym profile / schema version, when those tables exist.
    pub hint: Option<String>,
    relations: Vec<Relation>,
    filter: String,
    filter_mode: bool,
    /// Index into the filtered list.
    relation_index: usize,
    focus: Focus,
    page: Option<Page>,
    row_cursor: usize,
    col_cursor: usize,
    help: bool,
    esc_quit_pending: bool,
    /// Error or one-shot message. The footer prints it.
    pub status: String,
}

impl App {
    /// Empty browser labeled with the connection.
    pub fn new(label: String) -> Self {
        Self {
            label,
            hint: None,
            relations: Vec::new(),
            filter: String::new(),
            filter_mode: false,
            relation_index: 0,
            focus: Focus::Relations,
            page: None,
            row_cursor: 0,
            col_cursor: 0,
            help: false,
            esc_quit_pending: false,
            status: String::new(),
        }
    }

    /// Replace the relation list. Keeps the previous selection when that table is still visible.
    pub fn set_relations(&mut self, relations: Vec<Relation>) {
        let keep = self
            .selected_relation()
            .map(|rel| (rel.schema.clone(), rel.name.clone()));
        self.relations = relations;
        self.relation_index = 0;
        if let Some((schema, name)) = keep {
            let visible = self.visible_indices();
            if let Some(index) = visible
                .iter()
                .position(|&slot| self.relations[slot].schema == schema && self.relations[slot].name == name)
            {
                self.relation_index = index;
            }
        }
        self.clamp_relation();
        if self.selected_relation().is_none() {
            self.page = None;
            self.focus = Focus::Relations;
        }
    }

    /// Store the catalog hint.
    pub fn set_hint(&mut self, hint: Option<String>) {
        self.hint = hint;
    }

    /// Show `message` in the footer.
    pub fn set_status(&mut self, message: String) {
        self.status = message;
    }

    /// Accept a page. An empty page past the start is ignored so PgDn stops at the last rows.
    pub fn set_page(&mut self, page: Page) {
        if page.rows.is_empty() && page.offset > 0 {
            self.status = "end of rows".to_string();
            return;
        }
        self.row_cursor = 0;
        self.col_cursor = 0;
        self.page = Some(page);
        self.focus = Focus::Rows;
        if self.status == "end of rows" {
            self.status.clear();
        }
    }

    /// Relation under the cursor, if the filtered list is non-empty.
    pub fn selected_relation(&self) -> Option<&Relation> {
        let slot = self.visible_indices().get(self.relation_index).copied()?;
        self.relations.get(slot)
    }

    /// Handle one key. Filter mode and help swallow keys before pane movement.
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Action {
        if is_ctrl_q(code, modifiers) {
            return Action::Quit;
        }
        if self.help {
            if code == KeyCode::Esc || is_help_key(code, modifiers) {
                self.help = false;
                self.esc_quit_pending = false;
            }
            return Action::None;
        }
        if is_help_key(code, modifiers) {
            self.help = true;
            self.esc_quit_pending = false;
            return Action::None;
        }
        if self.filter_mode {
            return self.handle_filter_key(code, modifiers);
        }
        if code != KeyCode::Esc {
            self.clear_quit_arm();
        }
        match code {
            KeyCode::Esc => self.on_esc(),
            KeyCode::Char('/') => {
                self.filter.clear();
                self.filter_mode = true;
                self.relation_index = 0;
                Action::None
            }
            KeyCode::Char('r') if modifiers.is_empty() => Action::ReloadRelations,
            KeyCode::Enter if self.focus == Focus::Relations => Action::LoadPage { offset: 0 },
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = match self.focus {
                    Focus::Relations if self.page.is_some() => Focus::Rows,
                    _ => Focus::Relations,
                };
                Action::None
            }
            KeyCode::Char('j') | KeyCode::Down if modifiers.is_empty() => {
                self.move_cursor(1);
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up if modifiers.is_empty() => {
                self.move_cursor(-1);
                Action::None
            }
            KeyCode::Char('h') | KeyCode::Left if modifiers.is_empty() && self.focus == Focus::Rows => {
                self.col_cursor = self.col_cursor.saturating_sub(1);
                Action::None
            }
            KeyCode::Char('l') | KeyCode::Right if modifiers.is_empty() && self.focus == Focus::Rows => {
                let width = self.page.as_ref().map(|page| page.columns.len()).unwrap_or(0);
                if width > 0 && self.col_cursor + 1 < width {
                    self.col_cursor += 1;
                }
                Action::None
            }
            KeyCode::PageDown => self.page_rows(true),
            KeyCode::PageUp => self.page_rows(false),
            _ => Action::None,
        }
    }

    fn handle_filter_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Action {
        self.esc_quit_pending = false;
        match code {
            KeyCode::Esc => {
                self.filter.clear();
                self.filter_mode = false;
                self.relation_index = 0;
            }
            KeyCode::Enter => {
                self.filter_mode = false;
                self.relation_index = 0;
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.relation_index = 0;
            }
            KeyCode::Char(ch) if modifiers.is_empty() => {
                self.filter.push(ch);
                self.relation_index = 0;
            }
            _ => {}
        }
        Action::None
    }

    fn on_esc(&mut self) -> Action {
        if !self.filter.is_empty() {
            self.filter.clear();
            self.filter_mode = false;
            self.relation_index = 0;
            self.esc_quit_pending = false;
            return Action::None;
        }
        if self.focus == Focus::Rows {
            self.focus = Focus::Relations;
            self.esc_quit_pending = false;
            return Action::None;
        }
        if self.esc_quit_pending {
            return Action::Quit;
        }
        self.esc_quit_pending = true;
        self.status = "press Esc again to quit".to_string();
        Action::None
    }

    fn clear_quit_arm(&mut self) {
        if self.esc_quit_pending {
            self.esc_quit_pending = false;
            if self.status == "press Esc again to quit" {
                self.status.clear();
            }
        }
    }

    fn move_cursor(&mut self, delta: isize) {
        match self.focus {
            Focus::Relations => {
                let n = self.visible_len();
                if n == 0 {
                    return;
                }
                let next = self.relation_index as isize + delta;
                self.relation_index = next.clamp(0, n as isize - 1) as usize;
            }
            Focus::Rows => {
                let n = self.page.as_ref().map(|page| page.rows.len()).unwrap_or(0);
                if n == 0 {
                    return;
                }
                let next = self.row_cursor as isize + delta;
                self.row_cursor = next.clamp(0, n as isize - 1) as usize;
            }
        }
    }

    fn page_rows(&mut self, down: bool) -> Action {
        if self.focus != Focus::Rows {
            return Action::None;
        }
        let Some(page) = &self.page else {
            return Action::None;
        };
        if down {
            if page.rows.len() as u32 == PAGE_SIZE {
                Action::LoadPage {
                    offset: page.offset.saturating_add(PAGE_SIZE),
                }
            } else {
                self.status = "end of rows".to_string();
                Action::None
            }
        } else if page.offset == 0 {
            Action::None
        } else {
            Action::LoadPage {
                offset: page.offset.saturating_sub(PAGE_SIZE),
            }
        }
    }

    fn visible_indices(&self) -> Vec<usize> {
        let needle = self.filter.to_ascii_lowercase();
        self.relations
            .iter()
            .enumerate()
            .filter_map(|(index, relation)| {
                if needle.is_empty() || relation.label().to_ascii_lowercase().contains(&needle) {
                    Some(index)
                } else {
                    None
                }
            })
            .collect()
    }

    fn visible_len(&self) -> usize {
        self.visible_indices().len()
    }

    fn clamp_relation(&mut self) {
        let n = self.visible_len();
        if n == 0 {
            self.relation_index = 0;
        } else if self.relation_index >= n {
            self.relation_index = n - 1;
        }
    }

    /// Pane that receives movement keys.
    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// Help overlay is up.
    pub fn help(&self) -> bool {
        self.help
    }

    /// `/` filter is capturing keys.
    pub fn filter_mode(&self) -> bool {
        self.filter_mode
    }

    /// Current relation-name filter. Empty shows every table.
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Cursor into the filtered relation list.
    pub fn relation_index(&self) -> usize {
        self.relation_index
    }

    /// Cursor row inside the loaded page.
    pub fn row_cursor(&self) -> usize {
        self.row_cursor
    }

    /// Cursor column inside the loaded page. The grid draws from this column rightward.
    pub fn col_cursor(&self) -> usize {
        self.col_cursor
    }

    /// Loaded page, if Enter has opened a table.
    pub fn page(&self) -> Option<&Page> {
        self.page.as_ref()
    }

    /// Labels of relations that match the filter.
    pub fn visible_labels(&self) -> Vec<String> {
        self.visible_indices()
            .into_iter()
            .map(|index| self.relations[index].label())
            .collect()
    }
}

fn is_ctrl_q(code: KeyCode, modifiers: KeyModifiers) -> bool {
    modifiers.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('q') | KeyCode::Char('Q'))
}

fn is_help_key(code: KeyCode, modifiers: KeyModifiers) -> bool {
    modifiers.contains(KeyModifiers::ALT) && matches!(code, KeyCode::Char('?') | KeyCode::Char('/'))
}

#[cfg(test)]
mod tests {
    use crossterm::event::{
        KeyCode,
        KeyModifiers,
    };

    use super::{
        Action,
        App,
        Focus,
    };
    use crate::model::{
        Column,
        PAGE_SIZE,
        Page,
        Relation,
    };

    fn subjects() -> Relation {
        Relation {
            schema: "main".to_string(),
            name: "subjects".to_string(),
        }
    }

    fn sessions() -> Relation {
        Relation {
            schema: "main".to_string(),
            name: "sessions".to_string(),
        }
    }

    fn app_with_tables() -> App {
        let mut app = App::new("catalog.sqlite".to_string());
        app.set_relations(vec![subjects(), sessions()]);
        app
    }

    fn one_page(rows: usize, offset: u32) -> Page {
        Page {
            columns: vec![Column {
                name: "id".to_string(),
                type_name: "TEXT".to_string(),
                nullable: false,
            }],
            rows: vec![vec!["x".to_string()]; rows],
            offset,
            limit: PAGE_SIZE,
        }
    }

    #[test]
    fn filter_keeps_matching_names_and_swallows_movement_keys() {
        let mut app = app_with_tables();
        app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(app.filter(), "j");
        assert_eq!(app.relation_index(), 0);
        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(!app.filter_mode());
        assert_eq!(app.visible_labels(), vec!["sessions".to_string()]);
    }

    #[test]
    fn esc_backs_out_then_arms_quit() {
        let mut app = app_with_tables();
        app.set_page(one_page(1, 0));
        assert_eq!(app.focus(), Focus::Rows);
        assert_eq!(app.handle_key(KeyCode::Esc, KeyModifiers::NONE), Action::None);
        assert_eq!(app.focus(), Focus::Relations);
        assert_eq!(app.handle_key(KeyCode::Esc, KeyModifiers::NONE), Action::None);
        assert!(app.status.contains("Esc again"));
        assert_eq!(app.handle_key(KeyCode::Esc, KeyModifiers::NONE), Action::Quit);
    }

    #[test]
    fn esc_in_filter_clears_and_does_not_quit() {
        let mut app = app_with_tables();
        app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(app.handle_key(KeyCode::Esc, KeyModifiers::NONE), Action::None);
        assert!(app.filter().is_empty());
        assert!(!app.filter_mode());
        assert_eq!(app.handle_key(KeyCode::Esc, KeyModifiers::NONE), Action::None);
        assert_ne!(app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE), Action::Quit);
    }

    #[test]
    fn ctrl_q_quits_from_filter_and_help() {
        let mut app = app_with_tables();
        app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!(app.handle_key(KeyCode::Char('q'), KeyModifiers::CONTROL), Action::Quit);
        let mut app = app_with_tables();
        app.handle_key(KeyCode::Char('?'), KeyModifiers::ALT);
        assert!(app.help());
        app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(app.relation_index(), 0);
        assert_eq!(app.handle_key(KeyCode::Char('q'), KeyModifiers::CONTROL), Action::Quit);
    }

    #[test]
    fn page_down_requests_the_next_window_only_when_full() {
        let mut app = app_with_tables();
        app.set_page(one_page(PAGE_SIZE as usize, 0));
        assert_eq!(
            app.handle_key(KeyCode::PageDown, KeyModifiers::NONE),
            Action::LoadPage { offset: PAGE_SIZE }
        );
        app.set_page(one_page(3, 0));
        assert_eq!(app.handle_key(KeyCode::PageDown, KeyModifiers::NONE), Action::None);
        assert_eq!(app.status, "end of rows");
        assert_eq!(app.handle_key(KeyCode::PageUp, KeyModifiers::NONE), Action::None);
    }

    #[test]
    fn empty_tail_page_keeps_the_previous_rows() {
        let mut app = app_with_tables();
        app.set_page(one_page(PAGE_SIZE as usize, 0));
        app.set_page(one_page(0, PAGE_SIZE));
        assert_eq!(app.page().unwrap().offset, 0);
        assert_eq!(app.page().unwrap().rows.len(), PAGE_SIZE as usize);
        assert_eq!(app.status, "end of rows");
    }
}
