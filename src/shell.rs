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
            None => eprintln!("{target}: not found"),
        }
    }

    pub fn builtin_pwd(&self, ctx: &IoContext) {
        self.output(
            &ctx,
            &format!("{}", env::current_dir().unwrap_or_default().display()),
        );
    }

    pub fn builtin_cd(&self, args: &[&str], ctx: &IoContext) {
        let arg = args.first().copied().unwrap_or("");

        let home_path = if arg != "~" {
            PathBuf::from(arg)
        } else {
            self.get_home_path()
        };

        if let Err(_) = env::set_current_dir(&home_path) {
            self.output(
                &ctx,
                &format!("cd: {}: No such file or directory", home_path.display()),
            );
        }
    }

    pub fn execute(&self, command: &str, args: &[&str], ctx: &IoContext) {
        let Some(path_buf) = Shell::locate_executable(command) else {
            self.output(&ctx, &format!("{command}: not found"));
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
        if let Some(redirect_path) = &ctx.stdout_redirect {
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
    pub fn parse_context<'a>(args: &[&'a str]) -> (Vec<&'a str>, IoContext) {
        if args.iter().any(|&x| x == ">" || x == "1>") {
            let stdout_path = args.get(args.len() - 1).map(PathBuf::from);
            (
                args[..args.len() - 2].to_vec(),
                IoContext {
                    stdout_redirect: stdout_path,
                    stderr_redirect: None,
                },
            )
        } else {
            (
                args.to_vec(),
                IoContext {
                    stdout_redirect: None,
                    stderr_redirect: None,
                },
            )
        }
    }
}
