use is_executable::IsExecutable;
use rustyline::{
    Context, Helper,
    completion::{Completer, Pair},
    highlight::Highlighter,
    hint::Hinter,
    validate::Validator,
};
use std::{collections::BTreeSet, env, fs};

use crate::shell::Shell;

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

        let mut suggested_dir = None;
        let suggested_file: &str;
        match path_to_complete.rsplit_once("/") {
            Some((dir, f)) => {
                suggested_dir = Some(dir);
                suggested_file = f;
            }
            None => suggested_file = path_to_complete,
        }

        let mut candidates = Vec::new();

        if line_up_to_cursor[..start_pos].trim().is_empty() {
            //TODO extract the following code into locate command method

            for &builtin in Shell::BUILTINS {
                if builtin.starts_with(&suggested_file) {
                    candidates.push(Pair {
                        display: builtin.to_string(),
                        replacement: builtin.to_string() + " ",
                    })
                }
            }

            if let Some(path_var) = env::var_os("PATH") {
                let mut commands = BTreeSet::<String>::new();

                for path in env::split_paths(&path_var) {
                    if let Ok(entries) = fs::read_dir(&path) {
                        for entry in entries.flatten() {
                            if let Some(file_name) = entry.file_name().to_str() {
                                if !file_name.starts_with(&suggested_file) {
                                    continue;
                                }
                                if entry.path().is_file() && entry.path().is_executable() {
                                    commands.insert(file_name.to_string());
                                }
                            }
                        }
                    }
                }

                for name in commands {
                    candidates.push(Pair {
                        display: name.clone(),
                        replacement: format!("{name} "),
                    });
                }
            };
        }

        if let Ok(entries) = fs::read_dir(suggested_dir.unwrap_or(".")) {
            //TODO extract to locate files and dirs method

            let prefix = match suggested_dir {
                Some(p) => format!("{}/", p),
                None => String::new(),
            };

            for entry in entries.flatten() {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };

                if file_type.is_file()
                    && let Some(file_name) = entry.file_name().to_str()
                {
                    if file_name.starts_with(&suggested_file) {
                        candidates.push(Pair {
                            display: prefix.to_string() + file_name,
                            replacement: prefix.to_string() + file_name + " ",
                        });
                    }
                } else if file_type.is_dir()
                    && let Some(dir_name) = entry.file_name().to_str()
                {
                    if dir_name.starts_with(&suggested_file) {
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
