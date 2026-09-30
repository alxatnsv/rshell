use std::fs;

use rustyline::{
    Context, Helper,
    completion::{Completer, Pair},
    highlight::Highlighter,
    hint::Hinter,
    validate::Validator,
};

pub struct ShellHelper {}

impl ShellHelper {
    pub fn new() -> Self {
        Self {}
    }
}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let line_up_to_cursor = &line[..pos];
        let start_pos = line_up_to_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let word_to_complete = &line_up_to_cursor[start_pos..];

        let mut candidates = Vec::new();

        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.starts_with(word_to_complete) {
                        candidates.push(Pair {
                            display: file_name.to_string(),
                            replacement: file_name.to_string() + " ",
                        });
                    }
                }
            }
        }

        candidates.sort_by(|a, b| a.display.cmp(&b.display));

        Ok((start_pos, candidates))
    }
}

impl Hinter for ShellHelper {
    type Hint = String;
}
impl Highlighter for ShellHelper {}
impl Validator for ShellHelper {}
impl Helper for ShellHelper {}
