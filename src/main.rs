#[allow(unused_imports)]
use std::io::{self, Write};
use std::env;
use is_executable::IsExecutable;

#[derive(Debug)]
struct Shell {
    current_path: String,
}

impl Shell {
    const BUILTINS: &[&str] = &["echo", "exit", "type"];

    fn is_builtin(&self, cmd: &str) -> bool {
        Self::BUILTINS.contains(&cmd)
    }

    fn echo(&self, args: &str) {
        println!("{args}");
    }

    fn type_(&self, args: &str) {
        if self.is_builtin(args) {
            println!("{args} is a shell builtin");
            return;
        }

        let Some(path_var) = env::var_os("PATH") else {
            println!("{args}: not found");
            return;
        };

        for dir in env::split_paths(&path_var) {
            let candidate = dir.join(args);
            if candidate.is_executable() {
                println!("{args} is {}", candidate.display());
                    return;
            }
        }

        println!("{args}: not found");
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
            _ => println!("{}: command not found", command),
        }
    }
}
