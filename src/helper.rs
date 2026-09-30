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
        let path_to_complete = &line_up_to_cursor[start_pos..];

        let mut directory = None;
        let file: &str;
        match path_to_complete.rsplit_once("/") {
            Some((dir, f)) => {
                directory = Some(dir);
                file = f;
            },
            None => file = path_to_complete,
        }

        let mut candidates = Vec::new();

        if let Ok(entries) = fs::read_dir(directory.unwrap_or(".")) {
            let prefix = match directory {
                Some(p) => format!("{}/", p),
                None => String::new(),
            };

            for entry in entries.flatten() {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };

                if file_type.is_file() && let Some(file_name) = entry.file_name().to_str() {
                    if file_name.starts_with(&file) {
                        candidates.push(Pair {
                            display: prefix.to_string() + file_name,
                            replacement: prefix.to_string() + file_name + " ",
                        });
                    }
                }
                else if file_type.is_dir() && let Some(dir_name) = entry.file_name().to_str() {
                    if dir_name.starts_with(&file) {
                        candidates.push(Pair {
                            display: prefix.to_string() + dir_name + "/", 
                            replacement: prefix.to_string() + dir_name + "/",
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
