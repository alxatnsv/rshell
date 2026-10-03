use std::process::Child;

#[derive(Debug)]
pub struct Job {
    pub number: usize,
    pub child: Child,
    pub command_line: String,
}

impl Job {
    pub fn new(number: usize, child: Child, command_line: String) -> Self {
        Self { number, child, command_line }
    }
}
