#[derive(Debug)]
pub struct History {
    pub entries: Vec<String>,
}

impl History {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add_entry(&mut self, line: &str) {
        self.entries.push(line.to_string())
    }
}
