mod shell;

use shell::Shell;
#[allow(unused_imports)]
use std::io::{self, Write};

use crate::shell::IoContext;

fn main() {
    let shell = Shell {};

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();
        let splitted: Vec<&str> = line.trim().split_whitespace().collect();

        let Some(&command) = splitted.first() else {
            continue;
        };

        let (args, ctx) = IoContext::parse_context(&splitted[1..]);

        match command {
            "echo" => shell.builtin_echo(&args, &ctx),
            "type" => shell.builtin_type(&args, &ctx),
            "cd" => shell.builtin_cd(&args, &ctx),
            "pwd" => shell.builtin_pwd(&ctx),
            "exit" => break,

            _ => shell.execute(command, &args, &ctx)
        }
    }
}
