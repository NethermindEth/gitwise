use anyhow::Result;

pub struct App {
    pub title: String,
    pub content: String,
}

impl App {
    pub fn new(title: String, content: String) -> Self {
        Self { title, content }
    }

    pub fn update(&mut self) -> Result<()> {
        Ok(())
    }
}
