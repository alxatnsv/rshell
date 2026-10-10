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

use crate::completion_context::CompletionContext;
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

    pub fn remove_completion_spec(&self, cmd: &str) {
        self.completion_specs.borrow_mut().remove(cmd);
    }
}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let ctx = CompletionContext::parse(line, pos);
        let mut candidates = Vec::new();

        let Some(command) = ctx.command() else {
            return Ok((ctx.start_pos, candidates));
        };

        if ctx.is_command_position() {
            collect_command_candidates(ctx.file, &mut candidates);
        } else if let Some(script_path) = self.completion_specs.borrow().get(command) {
            collect_completion_spec_candidates(script_path, &ctx, &mut candidates);
        } else {
            collect_path_candidates(ctx.dir, ctx.file, &mut candidates);
        }

        candidates.sort_by(|a, b| a.display.cmp(&b.display));
        Ok((ctx.start_pos, candidates))
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

            if !file_name.starts_with(suggested_file) {
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

fn collect_completion_spec_candidates(script_path: &str, ctx: &CompletionContext, candidates: &mut Vec<Pair>) {
    let Some(command) = ctx.command() else { return };

    let Ok(output) = Command::new(script_path)
        .args([command, ctx.current_word, ctx.previous_word().unwrap_or("")])
        .env("COMP_LINE", ctx.line)
        .env("COMP_POINT", ctx.pos.to_string())
        .output()
    else {
        return;
    };

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines().filter(|l| l.starts_with(ctx.current_word)) {
        candidates.push(Pair {
            display: line.to_string(),
            replacement: format!("{line} "),
        });
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
