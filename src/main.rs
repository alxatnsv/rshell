use is_executable::IsExecutable;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::{env, os::unix::process::CommandExt, path::PathBuf, process::Command};

#[derive(Debug)]
struct Shell {
    // current_path: PathBuf,
}

impl Shell {
    const BUILTINS: &[&str] = &["echo", "exit", "type", "pwd", "cd"];

    fn echo(&self, args: &[&str]) {
        println!("{}", args.join(" "));
    }

    fn type_(&self, args: &[&str]) {
        let Some(&target) = args.first() else {
            return;
        };

        if self.is_builtin(target) {
            println!("{target} is a shell builtin");
            return;
        }

        match Shell::locate_executable(target) {
            Some(path) => println!("{target} is {}", path.display()),
            None => println!("{target}: not found"),
        }
    }

    fn pwd(&self) {
        println!("{}", env::current_dir().unwrap_or_default().display());
    }

    fn cd(&self, args: &[&str]) {
        let target = args.first().copied().unwrap_or("");

        if let Err(_) = env::set_current_dir(target) {
            println!("cd: {target}: No such file or directory");
        }
    }

    fn execute(&self, command: &str, args: &[&str]) {
        let Some(path_buf) = Shell::locate_executable(command) else {
            println!("{command}: not found");
            return;
        };

        let status = Command::new(path_buf).arg0(command).args(args).status();

        if let Err(e) = status {
            eprintln!("Failed to execute command: {e}");
        }
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
}

fn main() {
    let shell = Shell {
        // current_path: env::current_dir().unwrap_or_default(),
    };

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();
        let splitted: Vec<&str> = line.trim().split_whitespace().collect();

        let Some(&command) = splitted.first() else {
            continue;
        };

        let args = &splitted[1..];

        match command {
            "echo" => shell.echo(args),
            "type" => shell.type_(args),
            "cd" => shell.cd(args),
            "pwd" => shell.pwd(),
            "exit" => break,

            _ => shell.execute(command, args),
        }
    }
}
