use crate::app::{
    buffer::{Buffer, BufferPosition, RectilinearDirection as Rectilinear},
    theme::Theme,
};
use super::line_numbers::LineNumberType::{Absolute, Relative};
use super::line_numbers::LineNumbers;
use ratatui::{
    buffer::Buffer as TUI_Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Paragraph, StatefulWidget, Widget},
};
use std::{
    cmp::{max, min},
    iter::repeat,
    rc::Weak,
};

#[derive(Debug, Clone)]
pub struct TextWindow {
    buffer: Weak<Buffer>,
    theme: Weak<Theme>,
}

#[derive(Debug, Clone)]
pub struct ScreenBounds {
    top_line: usize,
    bottom_line: usize,
    leftmost_col: usize,
    rightmost_col: usize,
}

#[derive(Debug)]
pub struct TextWindowState {
    pub top_line: usize,
    pub leftmost_col: usize,
    pub last_height: usize,
    pub last_width: usize,
    pub cur_vertical_percent: f32,
    pub cursor: BufferPosition,
    pub last_manual_col: usize,
    buffer: Weak<Buffer>,
    theme: Weak<Theme>,
}

impl TextWindowState {
    pub fn new(buffer: Weak<Buffer>, theme: Weak<Theme>) -> Self {
        return TextWindowState {
            top_line: 0,
            leftmost_col: 0,
            last_height: 2,
            last_width: 2,
            cur_vertical_percent: 0.0,
            cursor: BufferPosition { line: 0, col: 0 },
            last_manual_col: 0,
            buffer,
            theme,
        };
    }

    pub fn move_cursor(&mut self, dir: Rectilinear) {
        match dir {
            Rectilinear::Up => {
                if self.cursor.line <= 0 {
                    return;
                }
                let mut relative_line = self.cursor.line - self.top_line;
                self.cursor.line -= 1;
                if self.cursor.line < self.top_line {
                    self.cur_vertical_percent = 0.0;
                    self.top_line = self.cursor.line;
                } else {
                    relative_line -= 1;
                    self.cur_vertical_percent =
                        relative_line as f32 / (self.last_height - 1) as f32;
                }

                let new_line_length = self.line_length(self.cursor.line);
                if self.cursor.col >= new_line_length {
                    self.jump_to_EOL();
                } else {
                    self.jump(&BufferPosition {
                        line: self.cursor.line,
                        col: min(self.last_manual_col, max(new_line_length, 1) - 1),
                    });
                }
            }

            Rectilinear::Down => {
                if self.cursor.line + 1 >= self.lines_count() {
                    return;
                }
                let mut relative_line = self.cursor.line - self.top_line;

                self.cursor.line += 1;
                // float comparison OK here because it is exact
                if self.cur_vertical_percent == 1.0 {
                    self.top_line += 1;
                } else {
                    relative_line += 1;
                    self.cur_vertical_percent =
                        relative_line as f32 / (self.last_height - 1) as f32;
                }

                let new_line_length = self.line_length(self.cursor.line);
                if self.cursor.col >= new_line_length {
                    self.jump_to_EOL();
                } else {
                    self.jump(&BufferPosition {
                        line: self.cursor.line,
                        col: min(self.last_manual_col, max(new_line_length, 1) - 1),
                    });
                }
            }

            Rectilinear::Right => {
                if self.lines_count() == 0 {
                    return;
                }
                let line_length = self.line_length(self.cursor.line);
                if self.cursor.col + 1 >= line_length {
                    return;
                }
                self.cursor.col += 1;
                self.last_manual_col = self.cursor.col;
                if self.cursor.col >= self.leftmost_col + self.last_width {
                    self.leftmost_col += 1;
                }
            }

            Rectilinear::Left => {
                if self.cursor.col <= 0 {
                    return;
                }
                self.cursor.col -= 1;
                self.last_manual_col = self.cursor.col;
                if self.cursor.col < self.leftmost_col {
                    self.leftmost_col = self.cursor.col;
                }
            }
        }
    }

    fn screen_bounds(&self) -> ScreenBounds {
        let top_line = self.top_line;
        let bottom_line = top_line + self.last_height - 1;
        let leftmost_col = self.leftmost_col;
        let rightmost_col = leftmost_col + self.last_width - 1;
        return ScreenBounds {
            top_line,
            bottom_line,
            leftmost_col,
            rightmost_col,
        };
    }

    fn is_on_screen(&self, pos: &BufferPosition) -> bool {
        let ScreenBounds {
            top_line,
            bottom_line,
            leftmost_col,
            rightmost_col,
        } = self.screen_bounds();
        let BufferPosition { line, col } = *pos;
        let within_vertically = line >= top_line && line <= bottom_line;
        let within_horizontally = col >= leftmost_col && col <= rightmost_col;
        return within_vertically && within_horizontally;
    }

    fn snap_to_EOL(&mut self) {
        if self.lines_count() == 0 {
            self.cursor.col = 0;
            self.leftmost_col = 0;
            return;
        }
        let line_length = self.line_length(self.cursor.line);
        if self.cursor.col >= line_length {
            self.jump_to_EOL();
        }
    }

    fn jump_within_screen(&mut self, pos: &BufferPosition) {
        self.cursor.line = pos.line;
        self.cursor.col = pos.col;
        let relative_line = pos.line - self.top_line;
        self.cur_vertical_percent = relative_line as f32 / (self.last_height - 1) as f32;
        self.snap_to_EOL();
    }

    pub fn jump(&mut self, pos: &BufferPosition) {
        if self.is_on_screen(pos) {
            self.jump_within_screen(pos);
            return;
        }
        let BufferPosition { line, col } = *pos;
        let ScreenBounds {
            top_line,
            bottom_line,
            leftmost_col,
            rightmost_col,
        } = self.screen_bounds();

        let vertically_out_of_bounds = line < top_line || line > bottom_line;
        if self.lines_count() > 0 && vertically_out_of_bounds {
            let line = min(line, self.lines_count() - 1);
            let relative_line = min(self.last_height / 2, line);
            self.cursor.line = line;
            self.cur_vertical_percent = relative_line as f32 / (self.last_height - 1) as f32;
        }

        if col < leftmost_col || col > rightmost_col {
            let relative_col = min(self.last_width * 3 / 4, col);
            self.leftmost_col = col - relative_col;
            self.cursor.col = col;
        }

        self.snap_to_EOL();
        self.last_manual_col = self.cursor.col;
    }

    pub fn jump_to_EOL(&mut self) {
        let line_length = self.line_length(self.cursor.line);
        if line_length == 0 {
            self.cursor.col = 0;
            self.leftmost_col = 0;
            return;
        }
        self.cursor.col = line_length - 1;
        let to_the_right = self.cursor.col >= self.leftmost_col + self.last_width;
        let out_of_bounds = to_the_right || self.cursor.col < self.leftmost_col;
        if !out_of_bounds {
            return;
        }
        if to_the_right {
            self.leftmost_col = self.cursor.col + 1 - self.last_width;
        } else if self.cursor.col >= self.last_width {
            self.leftmost_col = self.cursor.col + 1 - self.last_width;
        } else {
            self.leftmost_col = 0;
        }
    }

    pub fn jump_to_home(&mut self) {
        self.cursor.col = 0;
        self.leftmost_col = 0;
        self.last_manual_col = 0;
    }

    pub fn jump_to_last_line(&mut self) {
        let line = if self.lines_count() > 0 {
            self.lines_count() - 1
        } else {
            0
        };
        self.top_line = if line >= self.last_height {
            line - self.last_height + 1
        } else {
            0
        };
        self.cursor.line = line;
        let relative_line = line - self.top_line;
        self.cur_vertical_percent = relative_line as f32 / (self.last_height - 1) as f32;
        self.snap_to_EOL();
        self.last_manual_col = self.cursor.col;
    }

    fn lines_count(&self) -> usize {
        self.buffer
            .upgrade()
            .expect("counting lines in a dead buffer!")
            .lines
            .len()
    }

    fn line_length(&self, line: usize) -> usize {
        self.buffer
            .upgrade()
            .expect("checking line length in a dead buffer!")
            .lines[line]
            .len()
    }
}

impl TextWindow {
    pub fn new(buffer: Weak<Buffer>, theme: Weak<Theme>) -> TextWindow {
        TextWindow { buffer, theme }
    }

    fn build_lines(&self, height: u16, width: usize, state: &mut TextWindowState) -> Vec<Line> {
        let buffer = self
            .buffer
            .upgrade()
            .expect("building lines from a dead buffer!");
        let theme = self.theme.upgrade().expect("referencing dropped theme!");

        state.last_height = height.into();
        state.last_width = width;
        let cursor_rel_line: usize =
            (state.cur_vertical_percent * (height - 1) as f32).round() as usize;
        let top_line: usize = if state.cursor.line > cursor_rel_line {
            state.cursor.line - cursor_rel_line
        } else {
            0
        };
        let last_line: usize = min(top_line + height as usize, state.lines_count());
        let line_style = Style::default()
            .fg(theme.text_foreground)
            .bg(theme.text_background);
        return buffer.lines[top_line..last_line]
            .iter()
            .map(|line| {
                if state.leftmost_col < line.len() {
                    line[state.leftmost_col..].to_string()
                } else {
                    "".to_string()
                }
            })
            .map(|line| Line::styled(format!("{line: <width$}"), line_style))
            .collect();
    }

    fn highlight_cursor(&self, lines: &mut Vec<Line>, state: &mut TextWindowState) {
        if lines.is_empty() {
            lines.push(Line::from(String::from_iter(
                repeat(" ").take(state.last_width - 1),
            )));
        }

        if state.cursor.line < state.top_line {
            return;
        }
        let line = state.cursor.line - state.top_line;
        if line >= lines.len() {
            return;
        }

        let theme = self.theme.upgrade().expect("referencing dropped theme!");
        let col = state.cursor.col - state.leftmost_col;
        let line_style = Style::default()
            .bg(theme.selected_line_background)
            .fg(theme.selected_line_foreground);
        let cur_style = line_style.add_modifier(Modifier::REVERSED);

        let old_line: String = lines[line].to_owned().into();
        if old_line.is_empty() {
            lines[line] = Line::styled(" ", cur_style);
            return;
        }

        let left_span = Span::styled(old_line[..col].to_string(), line_style);
        let cur_span = Span::styled(old_line[col..col + 1].to_string(), cur_style);
        let right_span = Span::styled(old_line[col + 1..].to_string(), line_style);

        lines[line] = Line::from(vec![left_span, cur_span, right_span]);
    }
}

impl StatefulWidget for TextWindow {
    type State = TextWindowState;

    fn render(self, area: Rect, tui_buf: &mut TUI_Buffer, state: &mut Self::State) {
        let line_numbers_width: u16 = (format!("{}", state.lines_count()).chars().count() + 1)
            .try_into()
            .expect("line number too large!");
        let window_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Length(line_numbers_width),
                Constraint::Length(2),
                Constraint::Fill(1),
            ])
            .split(area);
        let theme = self.theme.upgrade().expect("referencing dropped theme!");
        let lines_area = window_layout[2];
        let mut lines = self.build_lines(lines_area.height, lines_area.width.into(), state);
        self.highlight_cursor(&mut lines, state);
        let line_numbers_area = window_layout[0];
        let line_hints_area = window_layout[1];
        let line_hints = Paragraph::new("").style(Style::default().bg(theme.text_background));
        let line_numbers = LineNumbers::new(
            Relative,
            state.top_line + 1,
            state.top_line + area.height as usize,
            state.cursor.line + 1,
        )
        .set_styles(
            theme.styles.line_numbers_normal,
            theme.styles.line_numbers_selected,
        );

        line_numbers.render(line_numbers_area, tui_buf);
        line_hints.render(line_hints_area, tui_buf);
        if lines.len() < lines_area.height as usize {
            let gap = lines_area.height - lines.len() as u16;
            let gap_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Fill(1), Constraint::Length(gap)])
                .split(area)[1];
            Block::new()
                .bg(theme.text_background)
                .fg(theme.text_background)
                .render(gap_area, tui_buf);
        }
        Paragraph::new(lines).render(lines_area, tui_buf);
    }
}