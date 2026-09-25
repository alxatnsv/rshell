mod shell;
mod io_context;
mod history;

use shell::Shell;
use io_context::IoContext;
#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    let mut shell = Shell::new();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();
        let splitted: Vec<&str> = line.trim().split_whitespace().collect();

        let Some(&command) = splitted.first() else {
            continue;
        };

        shell.add_entry(&line.trim());

        let (args, ctx) = IoContext::parse_context(&splitted[1..]);

        match command {
            "echo" => shell.builtin_echo(&args, &ctx),
            "type" => shell.builtin_type(&args, &ctx),
            "cd" => shell.builtin_cd(&args, &ctx),
            "pwd" => shell.builtin_pwd(&ctx),
            "history" => shell.builtin_history(&args, &ctx),
            "exit" => break,

            _ => shell.execute(command, &args, &ctx)
        }
    }
}
