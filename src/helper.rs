use is_executable::IsExecutable;
use rustyline::{
    Context, Helper,
    completion::{Completer, Pair},
    highlight::Highlighter,
    hint::Hinter,
    validate::Validator,
};
use std::{
    cell::{Ref, RefCell},
    collections::{BTreeSet, HashMap},
    env, fs,
    process::Command,
};

use crate::shell::Shell;

pub struct ShellHelper {
    completion_specs: RefCell<HashMap<String, String>>,
}

impl ShellHelper {
    pub fn new() -> Self {
        Self {
            completion_specs: RefCell::new(HashMap::new()),
        }
    }

    pub fn get_completion_spec(&self) -> Ref<'_, HashMap<String, String>> {
        self.completion_specs.borrow()
    }

    pub fn set_completion_spec(&self, cmd: &str, path: &str) {
        self.completion_specs.borrow_mut().insert(cmd.to_string(), path.to_string());
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

        let ends_with_space = line_up_to_cursor.ends_with(char::is_whitespace);
        let words: Vec<&str> = line_up_to_cursor.split_whitespace().collect();

        if words.is_empty() {
            return Ok((start_pos, candidates));
        }

        let is_command_position = words.len() <= 1 && !ends_with_space;

        if is_command_position {
            collect_command_candidates(suggested_file, &mut candidates);
        } else {
            let command = words[0];

            if let Some(script_path) = self.completion_specs.borrow().get(command) {
                let previous_word: Option<&str>;
                match words.len() {
                    3 => previous_word = Some(words[1]),
                    _ => previous_word = None,
                }

                collect_completion_spec_candidates(command, script_path, suggested_file, &mut candidates, previous_word);
            } else {
                collect_path_candidates(suggested_dir, suggested_file, &mut candidates);
            }
        }

        candidates.sort_by(|a, b| a.display.cmp(&b.display));

        Ok((start_pos, candidates))
    }
}

fn collect_command_candidates(suggested_file: &str, candidates: &mut Vec<Pair>) {
    for &builtin in Shell::BUILTINS {
        if builtin.starts_with(suggested_file) {
            candidates.push(Pair {
                display: builtin.to_string(),
                replacement: builtin.to_string() + " ",
            })
        }
    }

    let Some(path_var) = env::var_os("PATH") else {
        return;
    };

    let mut commands = BTreeSet::<String>::new();

    for path in env::split_paths(&path_var) {
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };

        for entry in entries.flatten() {
            let os_file_name = entry.file_name();
            let Some(file_name) = os_file_name.to_str() else {
                continue;
            };

            if !file_name.starts_with(&suggested_file) {
                continue;
            }
            let path_buf = entry.path();
            if path_buf.is_file() && path_buf.is_executable() {
                commands.insert(file_name.to_string());
            }
        }
    }

    for name in commands {
        candidates.push(Pair {
            display: name.clone(),
            replacement: format!("{name} "),
        });
    }
}

fn collect_completion_spec_candidates(command: &str, script_path: &str, suggested_arg: &str, candidates: &mut Vec<Pair>, previous_word: Option<&str>) {
    let mut cmd = Command::new(script_path);
    let cmd_args = vec![command, suggested_arg, previous_word.unwrap_or("")];
    cmd.args(cmd_args);

    let Ok(output) = cmd.output() else {
        return;
    };
    let text = String::from_utf8_lossy(&output.stdout);

    for line in text.lines() {
        if line.starts_with(suggested_arg) {
            candidates.push(Pair {
                display: line.to_string(),
                replacement: line.to_string() + " ",
            });
        }
    }
}

fn collect_path_candidates(suggested_dir: Option<&str>, suggested_file: &str, candidates: &mut Vec<Pair>) {
    if let Ok(entries) = fs::read_dir(suggested_dir.unwrap_or(".")) {
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
                if file_name.starts_with(suggested_file) {
                    candidates.push(Pair {
                        display: prefix.to_string() + file_name,
                        replacement: prefix.to_string() + file_name + " ",
                    });
                }
            } else if file_type.is_dir()
                && let Some(dir_name) = entry.file_name().to_str()
            {
                if dir_name.starts_with(suggested_file) {
                    candidates.push(Pair {
                        display: prefix.to_string() + dir_name + "/",
                        replacement: prefix.to_string() + dir_name + "/",
                    });
                }
            }
        }
    }
}

impl Hinter for ShellHelper {
    type Hint = String;
}
impl Highlighter for ShellHelper {}
impl Validator for ShellHelper {}
impl Helper for ShellHelper {}
