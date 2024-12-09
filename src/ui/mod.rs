use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

mod app;
mod widgets;

pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl Tui {
    /// Create a new TUI
    pub fn new() -> Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        enable_raw_mode()?;
        
        Ok(Self { terminal })
    }

    /// Run the TUI with the given content
    pub fn run(&mut self, title: &str, content: &str) -> Result<()> {
        self.terminal.clear()?;
        
        loop {
            self.terminal.draw(|frame| {
                widgets::draw_main_layout(frame, title, content);
            })?;
            
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
        
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        disable_raw_mode().unwrap();
        self.terminal.show_cursor().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_creation() {
        let tui = Tui::new();
        assert!(tui.is_ok());
    }
}
