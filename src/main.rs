use is_executable::IsExecutable;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::{
    env, os::unix::process::CommandExt, path::PathBuf, process::Command,
};

#[derive(Debug)]
struct Shell {
    current_path: String,
}

impl Shell {
    const BUILTINS: &[&str] = &["echo", "exit", "type"];
    
    fn echo(&self, args: &str) {
        println!("{args}");
    }

    fn type_(&self, command: &str) {
        if self.is_builtin(command) {
            println!("{command} is a shell builtin");
            return;
        }

        match Shell::locate_executable(command) {
            Some(path) => println!("{command} is {}", path.display()),
            None => println!("{command}: not found"),
        }
    }

    fn execute(&self, command: &str, args: &str) {
        let Some(path_buf) = Shell::locate_executable(command) else {
            println!("{command}: not found");
            return;
        };

        let status = Command::new(path_buf)
            .arg0(command)
            .args(args.split(" "))
            .status();

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
            if candidate.is_executable() {
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
        current_path: String::from(""),
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

        let args = &splitted[1..].join(" ");

        match command {
            "echo" => shell.echo(args),
            "type" => shell.type_(args),
            "exit" => break,

            _ => shell.execute(command, args),
        }
    }
}
