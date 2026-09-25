use crate::history::History;
use crate::io_context::IoContext;
use is_executable::IsExecutable;

#[allow(unused_imports)]
use std::io::{self, Write};
use std::{
    env,
    fs::{File, OpenOptions},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::Command,
};

#[derive(Debug)]
pub struct Shell {
    history: History,
}

impl Shell {
    pub fn new() -> Self {
        Self { history: History::new() }
    }

    const BUILTINS: &[&str] = &["echo", "exit", "type", "pwd", "cd", "history"];

    pub fn builtin_echo(&self, args: &[&str], ctx: &IoContext) {
        self.output(ctx, &args.join(" "));
    }

    pub fn builtin_history(&mut self, args: &[&str], ctx: &IoContext) {
        let len = self.history.entries.len();
        let mut limit = len;

        if let Some(first_arg) = args.get(0) {
            if let Ok(num) = first_arg.parse::<usize>() {
                limit = num.min(self.history.entries.len());
            }
        }

        let start_idx = len - limit;

        for (i, entry) in self.history.entries[start_idx..].iter().enumerate() {
            self.output(ctx, &format!("{:>5}  {}", start_idx + i + 1, entry));
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
            if let Ok(file) = self.open_or_create(&ctx, file_path) {
                cmd.stdout(file);
            }
        }

        if let Some(file_path) = &ctx.stderr_redirect {
            if let Ok(file) = self.open_or_create(&ctx, file_path) {
                cmd.stderr(file);
            }
        }

        if let Err(e) = cmd.status() {
            eprintln!("Failed to execute command: {e}");
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
            if let Ok(mut f) = self.open_or_create(&ctx, redirect_path) {
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
            if let Ok(mut f) = self.open_or_create(&ctx, redirect_path) {
                let _ = writeln!(f, "{args}");
            }
        } else {
            println!("{args}");
        }
    }

    fn open_or_create(&self, ctx: &IoContext, path: &PathBuf) -> Result<File, std::io::Error> {
        if ctx.append {
            OpenOptions::new().create(true).append(true).open(path)
        } else {
            File::create(path)
        }
    }

    pub fn add_entry(&mut self, line: &str) {
        self.history.add_entry(line);
    }
}
