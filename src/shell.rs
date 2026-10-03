use crate::{helper::ShellHelper, io_context::IoContext, job::Job};
use is_executable::IsExecutable;
use rustyline::{
    Editor,
    history::{DefaultHistory, History},
};

#[allow(unused_imports)]
use std::io::{self, Write};
use std::{
    cell::{Cell, Ref, RefCell},
    collections::HashMap,
    env,
    fs::{File, OpenOptions},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::Command,
};

#[derive(Debug)]
pub struct Shell {
    history_sync_idx: Cell<usize>,
    variables: RefCell<HashMap<String, String>>,
    jobs: RefCell<Vec<Job>>,
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            history_sync_idx: Cell::new(0),
            variables: RefCell::new(HashMap::new()),
            jobs: RefCell::new(Vec::new()),
        }
    }

    pub const BUILTINS: &[&str] = &["echo", "exit", "type", "pwd", "cd", "history", "declare", "complete", "jobs"];

    pub fn get_variables(&self) -> Ref<'_, HashMap<String, String>> {
        self.variables.borrow()
    }

    pub fn builtin_echo(&self, args: &[&str], ctx: &IoContext) {
        self.output(ctx, &args.join(" "));
    }

    pub fn builtin_complete(&self, args: &[&str], ctx: &IoContext, helper: &ShellHelper) {
        let Some(param) = args.get(0) else {
            return;
        };

        match param {
            &"-p" => {
                if let Some(cmd) = args.get(1) {
                    let compls = helper.get_completion_spec();
                    let Some(path) = compls.get(*cmd) else {
                        self.error(ctx, &format!("complete: {cmd}: no completion specification",));
                        return;
                    };
                    self.output(ctx, &format!("complete -C '{path}' {cmd}"));
                }
            }
            &"-C" => {
                if let (Some(path), Some(cmd)) = (args.get(1), args.get(2)) {
                    helper.set_completion_spec(cmd, path);
                }
            }
            &"-r" => {
                if let Some(cmd) = args.get(1) {
                    helper.remove_completion_spec(cmd);
                }
            }
            _ => {}
        }
    }

    pub fn builtin_jobs(&self, ctx: &IoContext) {
        if self.jobs.borrow().is_empty() {
            return;
        }
        self.output(ctx, &format!("[1]+ Running                 {}", self.jobs.borrow()[0].command_line));
    }

    pub fn builtin_history(&self, args: &[&str], ctx: &IoContext, rl: &mut Editor<ShellHelper, DefaultHistory>) {
        let len = rl.history().len();
        let mut limit = len;

        if let Some(input_num) = args.get(0).and_then(|arg| arg.parse::<usize>().ok()) {
            limit = input_num.min(len);
        }

        if let Some(file_path) = args.get(1).map(PathBuf::from) {
            let action = args[0];
            self.history_action(&action, &file_path, &ctx, rl);
            return;
        }

        let start_idx = len - limit;

        for (i, entry) in rl.history().iter().skip(start_idx).enumerate() {
            self.output(ctx, &format!("{:>5}  {}", start_idx + i + 1, entry));
        }
    }

    pub fn builtin_declare(&self, args: &[&str], ctx: &IoContext) {
        let Some(param) = args.get(0) else {
            return;
        };

        match param {
            &"-p" => {
                if let Some(key) = args.get(1) {
                    let vars = self.variables.borrow();
                    let Some(value) = vars.get(*key) else {
                        self.error(&ctx, &format!("declare: {key}: not found"));
                        return;
                    };
                    self.output(&ctx, &format!("declare -- {key}=\"{value}\""));
                }
            }
            _ => {
                if let Some((key, value)) = param.split_once("=") {
                    let true = self.is_valid_var_identifier(key) else {
                        self.error(&ctx, &format!("declare: `{key}={value}': not a valid identifier"));
                        return;
                    };
                    self.set_variable(key, value);
                }
            }
        }
    }

    pub fn builtin_type(&self, args: &[&str], ctx: &IoContext) {
        let Some(&target) = args.first() else {
            return;
        };

        if self.is_builtin(target) {
            self.output(&ctx, &format!("{target} is a shell builtin"));
            return;
        }

        match Shell::locate_executable(target) {
            Some(path) => self.output(&ctx, &format!("{target} is {}", path.display())),
            None => self.error(&ctx, &format!("{target}: not found")),
        }
    }

    pub fn builtin_pwd(&self, ctx: &IoContext) {
        self.output(&ctx, &format!("{}", env::current_dir().unwrap_or_default().display()));
    }

    pub fn builtin_cd(&self, args: &[&str], ctx: &IoContext) {
        let arg = args.first().copied().unwrap_or("");

        let home_path = if arg != "~" { PathBuf::from(arg) } else { self.get_home_path() };

        if let Err(_) = env::set_current_dir(&home_path) {
            self.error(&ctx, &format!("cd: {}: No such file or directory", home_path.display()));
        }
    }

    pub fn execute(&self, command: &str, args: &[&str], ctx: &IoContext) {
        let Some(path_buf) = Shell::locate_executable(command) else {
            self.error(&ctx, &format!("{command}: not found"));
            return;
        };

        let mut cmd = Command::new(path_buf);
        cmd.arg0(command).args(args);

        if let Some(file_path) = &ctx.stdout_redirect {
            if let Ok(file) = self.open_or_create(ctx.append, file_path) {
                cmd.stdout(file);
            }
        }

        if let Some(file_path) = &ctx.stderr_redirect {
            if let Ok(file) = self.open_or_create(ctx.append, file_path) {
                cmd.stderr(file);
            }
        }

        if ctx.background {
            if let Ok(child) = cmd.spawn() {
                let command_line = if args.is_empty() { format!("{command} &") } else { format!("{command} {} &", args.join(" ")) };
                let job = Job::new(self.jobs.borrow().len() + 1, child, command_line);
                println!("[{}] {}", job.number, job.child.id());
                self.jobs.borrow_mut().push(job);
            }
        } else {
            let _ = cmd.status();
        }
    }

    fn get_home_path(&self) -> PathBuf {
        let Some(home) = env::var_os("HOME") else {
            return PathBuf::new();
        };

        PathBuf::from(home)
    }

    fn locate_executable(command: &str) -> Option<PathBuf> {
        let Some(path_var) = env::var_os("PATH") else {
            return None;
        };

        for dir in env::split_paths(&path_var) {
            let candidate = dir.join(command);
            if candidate.is_file() && candidate.is_executable() {
                return Some(candidate);
            }
        }

        None
    }

    fn is_builtin(&self, cmd: &str) -> bool {
        Self::BUILTINS.contains(&cmd)
    }

    fn output(&self, ctx: &IoContext, args: &str) {
        if let Some(redirect_path) = &ctx.stderr_redirect {
            let _ = File::create(redirect_path);
        }

        if let Some(redirect_path) = &ctx.stdout_redirect {
            if let Ok(mut f) = self.open_or_create(ctx.append, redirect_path) {
                let _ = writeln!(f, "{args}");
            }
        } else {
            println!("{args}");
        }
    }

    fn error(&self, ctx: &IoContext, args: &str) {
        if let Some(redirect_path) = &ctx.stdout_redirect {
            let _ = File::create(redirect_path);
        }

        if let Some(redirect_path) = &ctx.stderr_redirect {
            if let Ok(mut f) = self.open_or_create(ctx.append, redirect_path) {
                let _ = writeln!(f, "{args}");
            }
        } else {
            println!("{args}");
        }
    }

    fn open_or_create(&self, append: bool, path: &PathBuf) -> Result<File, std::io::Error> {
        if append { OpenOptions::new().create(true).append(true).open(path) } else { File::create(path) }
    }

    fn history_action(&self, action: &str, file_path: &PathBuf, ctx: &IoContext, rl: &mut Editor<ShellHelper, DefaultHistory>) {
        match action {
            "-r" => {
                let _ = rl.load_history(&file_path);
                self.history_sync_idx.set(rl.history().len());
                return;
            }
            "-w" => {
                if let Ok(mut f) = self.open_or_create(false, &file_path) {
                    rl.history().iter().for_each(|line| writeln!(f, "{}", line).unwrap());
                }

                self.history_sync_idx.set(rl.history().len());
                return;
            }
            "-a" => {
                let sync_idx = self.history_sync_idx.get();
                if sync_idx < rl.history().len() {
                    if let Ok(mut f) = self.open_or_create(true, &file_path) {
                        for line in rl.history().iter().skip(sync_idx) {
                            writeln!(f, "{}", line).unwrap();
                        }
                    }

                    self.history_sync_idx.set(rl.history().len());
                }
                return;
            }
            _ => {
                self.error(&ctx, &format!("{action}: invalid option"));
                return;
            }
        }
    }

    fn set_variable(&self, key: &str, value: &str) {
        self.variables.borrow_mut().insert(key.to_string(), value.to_string());
    }

    fn is_valid_var_identifier(&self, name: &str) -> bool {
        let Some((&first, rest)) = name.as_bytes().split_first() else {
            return false;
        };

        (first.is_ascii_alphabetic() || first == b'_') && rest.iter().all(|&b| b.is_ascii_alphanumeric() || b == b'_')
    }
}
