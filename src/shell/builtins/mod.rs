use crate::{helper::ShellHelper, streams::Streams};
use rustyline::{
    Editor,
    history::{DefaultHistory, History},
};
use std::io::Write;
use std::{env, path::PathBuf};

use super::Shell;

impl Shell {
    pub fn builtin_echo(&self, args: &[&str], io: &mut Streams) {
        let _ = writeln!(io.out, "{}", args.join(" "));
    }

    pub fn builtin_complete(&self, args: &[&str], io: &mut Streams, helper: &ShellHelper) {
        let Some(param) = args.first() else {
            return;
        };

        match *param {
            "-p" => {
                if let Some(cmd) = args.get(1) {
                    let compls = helper.get_completion_spec();
                    let Some(path) = compls.get(*cmd) else {
                        let _ = writeln!(io.err, "complete: {cmd}: no completion specification");
                        return;
                    };
                    let _ = writeln!(io.out, "complete -C '{path}' {cmd}");
                }
            }
            "-C" => {
                if let (Some(path), Some(cmd)) = (args.get(1), args.get(2)) {
                    helper.set_completion_spec(cmd, path);
                }
            }
            "-r" => {
                if let Some(cmd) = args.get(1) {
                    helper.remove_completion_spec(cmd);
                }
            }
            _ => {}
        }
    }

    pub fn builtin_jobs(&self, io: &mut Streams) {
        let mut jobs = self.jobs.borrow_mut();
        let len = jobs.len();

        for (idx, job) in jobs.iter_mut().enumerate() {
            let sign = if idx + 1 == len {
                '+'
            } else if idx + 2 == len {
                '-'
            } else {
                ' '
            };

            let done = job.is_done();
            job.running = !done;

            let (status, command) = if done {
                ("Done", job.command_line.clone())
            } else {
                ("Running", format!("{} &", job.command_line))
            };

            let _ = writeln!(io.out, "[{}]{}  {:<24}{}", job.number, sign, status, command);
        }
        jobs.retain(|x| x.running);
    }

    pub fn builtin_history(&self, args: &[&str], io: &mut Streams, rl: &mut Editor<ShellHelper, DefaultHistory>) {
        let len = rl.history().len();
        let mut limit = len;

        if let Some(input_num) = args.first().and_then(|arg| arg.parse::<usize>().ok()) {
            limit = input_num.min(len);
        }

        if let Some(file_path) = args.get(1).map(PathBuf::from) {
            let action = args[0];
            self.history_action(action, &file_path, io, rl);
            return;
        }

        let start_idx = len - limit;

        for (i, entry) in rl.history().iter().skip(start_idx).enumerate() {
            let _ = writeln!(io.out, "{:>5}  {}", start_idx + i + 1, entry);
        }
    }

    pub fn builtin_declare(&self, args: &[&str], io: &mut Streams) {
        let Some(param) = args.first() else {
            return;
        };

        match param {
            &"-p" => {
                if let Some(key) = args.get(1) {
                    let vars = self.variables.borrow();
                    let Some(value) = vars.get(*key) else {
                        let _ = writeln!(io.err, "declare: {key}: not found");
                        return;
                    };
                    let _ = writeln!(io.out, "declare -- {key}=\"{value}\"");
                }
            }
            _ => {
                if let Some((key, value)) = param.split_once("=") {
                    let true = self.is_valid_var_identifier(key) else {
                        let _ = writeln!(io.err, "declare: `{key}={value}': not a valid identifier");
                        return;
                    };
                    self.set_variable(key, value);
                }
            }
        }
    }

    pub fn builtin_type(&self, args: &[&str], io: &mut Streams) {
        let Some(&target) = args.first() else { return };

        if self.is_builtin(target) {
            let _ = writeln!(io.out, "{target} is a shell builtin");
            return;
        }

        match Shell::locate_executable(target) {
            Some(path) => {
                let _ = writeln!(io.out, "{target} is {}", path.display());
            }
            None => {
                let _ = writeln!(io.err, "{target}: not found");
            }
        }
    }

    pub fn builtin_pwd(&self, io: &mut Streams) {
        let _ = writeln!(io.out, "{}", env::current_dir().unwrap_or_default().display());
    }

    pub fn builtin_cd(&self, args: &[&str], io: &mut Streams) {
        let arg = args.first().copied().unwrap_or("");

        let home_path = if arg != "~" { PathBuf::from(arg) } else { self.get_home_path() };

        if env::set_current_dir(&home_path).is_err() {
            let _ = writeln!(io.err, "cd: {}: No such file or directory", home_path.display());
        }
    }

    fn history_action(&self, action: &str, file_path: &PathBuf, io: &mut Streams, rl: &mut Editor<ShellHelper, DefaultHistory>) {
        match action {
            "-r" => {
                let _ = rl.load_history(&file_path);
                self.history_sync_idx.set(rl.history().len());
            }
            "-w" => {
                if let Ok(mut f) = self.open_or_create(false, file_path) {
                    let _ = rl.history().iter().for_each(|line| writeln!(f, "{}", line).unwrap());
                }

                self.history_sync_idx.set(rl.history().len());
            }
            "-a" => {
                let sync_idx = self.history_sync_idx.get();
                if sync_idx < rl.history().len() {
                    if let Ok(mut f) = self.open_or_create(true, file_path) {
                        for line in rl.history().iter().skip(sync_idx) {
                            writeln!(f, "{}", line).unwrap();
                        }
                    }

                    self.history_sync_idx.set(rl.history().len());
                }
            }
            _ => {
                let _ = writeln!(io.err, "{action}: invalid option");
            }
        }
    }

    fn is_valid_var_identifier(&self, name: &str) -> bool {
        let Some((&first, rest)) = name.as_bytes().split_first() else {
            return false;
        };

        (first.is_ascii_alphabetic() || first == b'_') && rest.iter().all(|&b| b.is_ascii_alphanumeric() || b == b'_')
    }

    fn is_builtin(&self, cmd: &str) -> bool {
        Self::BUILTINS.contains(&cmd)
    }

    fn set_variable(&self, key: &str, value: &str) {
        self.variables.borrow_mut().insert(key.to_string(), value.to_string());
    }
}
