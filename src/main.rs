mod io_context;
mod shell;

#[allow(unused_imports)]
use io_context::IoContext;
use rustyline::{Config, DefaultEditor};
use shell::Shell;

fn main() {
    let shell = Shell::new();

    let config = Config::builder().history_ignore_dups(false).unwrap().build();
    let mut rl = DefaultEditor::with_config(config).unwrap();

    loop {
        let Ok(line) = rl.readline("$ ") else {
            continue;
        };

        let splitted: Vec<&str> = line.split_whitespace().collect();
        let Some(&command) = splitted.first() else {
            continue;
        };

        let _ = rl.add_history_entry(line.as_str());
        let (args, ctx) = IoContext::parse_context(&splitted[1..]);

        match command {
            "echo" => shell.builtin_echo(&args, &ctx),
            "type" => shell.builtin_type(&args, &ctx),
            "cd" => shell.builtin_cd(&args, &ctx),
            "pwd" => shell.builtin_pwd(&ctx),
            "history" => shell.builtin_history(&args, &ctx, &mut rl),
            "exit" => break,

            _ => shell.execute(command, &args, &ctx),
        }
    }
}
