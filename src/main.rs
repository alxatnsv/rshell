#[allow(unused_imports)]
use std::io::{self, Write};

const BUILTINS: &[&str] = &["echo", "exit", "type"];

fn is_builtin(cmd: &str) -> bool {
    BUILTINS.contains(&cmd)
}

fn main() {
    loop {

        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        let command = command.trim();

        if command == "exit" {
            break;
        } else if command.starts_with("echo ") {
            println!("{}", &command[5..]);
        } else if command.starts_with("type "){
            if is_builtin(&command[5..]) {
                println!("{} is a shell builtin", &command[5..]);
            }
            else {
                println!("{}: not found", &command[5..]);
            }
        } else {
            println!("{}: command not found", command);
        }

    }
}
