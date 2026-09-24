use is_executable::IsExecutable;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::{env, fs::File, os::unix::process::CommandExt, path::PathBuf, process::Command};

#[derive(Debug)]
pub struct Shell {
    // current_path: PathBuf,
}

impl Shell {
    const BUILTINS: &[&str] = &["echo", "exit", "type", "pwd", "cd"];

    pub fn builtin_echo(&self, args: &[&str], ctx: &IoContext) {
        self.output(ctx, &args.join(" "));
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
            if let Ok(file) = File::create(file_path) {
                cmd.stdout(file);
            }
        }

        if let Some(file_path) = &ctx.stderr_redirect {
            if let Ok(file) = File::create(file_path) {
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
            if let Ok(mut f) = File::create(redirect_path) {
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
            if let Ok(mut f) = File::create(redirect_path) {
                let _ = writeln!(f, "{args}");
            }
        } else {
            println!("{args}");
        }
    }
}

pub struct IoContext {
    stdout_redirect: Option<PathBuf>,
    stderr_redirect: Option<PathBuf>,
}

impl IoContext {
    const REDIRECT_OPERATORS: &'static [&'static str] = &[">", "1>", "2>", ">>", "1>>", "2>>", "&>"];

    pub fn parse_context<'a>(args: &[&'a str]) -> (Vec<&'a str>, IoContext) {
        if let Some(op_pos) = args.iter().position(|&x| Self::is_redirect_op(x)) {
            if op_pos + 1 < args.len() {
                let redirect_op = args[op_pos];
                let file_path = PathBuf::from(args[op_pos + 1]);

                let (stdout_redirect, stderr_redirect) = match redirect_op {
                    ">" | "1>" | ">>" | "1>>" => (Some(file_path), None),
                    "2>" | "2>>" => (None, Some(file_path)),
                    _ => (None, None),
                };

                return (args[..op_pos].to_vec(), IoContext { stdout_redirect, stderr_redirect });
            }
        }

        (
            args.to_vec(),
            IoContext {
                stdout_redirect: None,
                stderr_redirect: None,
            },
        )
    }

    fn is_redirect_op(s: &str) -> bool {
        Self::REDIRECT_OPERATORS.contains(&s)
    }
}
