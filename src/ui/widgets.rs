use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw_main_layout(frame: &mut Frame, title: &str, content: &str) {
    let size = frame.size();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
        ].as_ref())
        .split(size);
    
    let title_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    
    let content_block = Block::default()
        .borders(Borders::ALL);
    
    let content_widget = Paragraph::new(content)
        .block(content_block)
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    frame.render_widget(title_block, chunks[0]);
    frame.render_widget(content_widget, chunks[1]);
}
