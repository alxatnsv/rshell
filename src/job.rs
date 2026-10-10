use std::process::Child;

#[derive(Debug)]
pub struct Job {
    pub number: usize,
    pub children: Vec<Child>,
    pub command_line: String,
    pub running: bool,
}

impl Job {
    pub fn new(number: usize, children: Vec<Child>, command_line: String) -> Self {
        Self {
            number,
            children,
            command_line,
            running: true,
        }
    }

    pub fn last_pid(&self) -> Option<u32> {
        self.children.last().map(|c| c.id())
    }

    pub fn is_done(&mut self) -> bool {
        let mut done = true;
        for child in &mut self.children {
            if matches!(child.try_wait(), Ok(None)) {
                done = false;
            }
        }
        done
    }
}
