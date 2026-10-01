mod helper;
mod io_context;
mod lexer;
mod shell;

#[allow(unused_imports)]
use helper::ShellHelper;
use io_context::IoContext;
use lexer::Lexer;
use rustyline::{Config, Editor, history::DefaultHistory};
use shell::Shell;
use std::io::Write;
use std::{env, fs::File};

fn main() {
    let shell = Shell::new();

    let config = Config::builder().history_ignore_dups(false).unwrap().completion_type(rustyline::CompletionType::List).build();
    let mut rl = Editor::<ShellHelper, DefaultHistory>::with_config(config).unwrap();
    let helper = ShellHelper::new();
    rl.set_helper(Some(helper));
    if let Some(hist_env) = env::var_os("HISTFILE") {
        let _ = rl.load_history(&hist_env);
    };

    loop {
        let Ok(line) = rl.readline("$ ") else {
            continue;
        };

        let tokens = {
            let vars = shell.get_variables();
            match Lexer::new(&vars).tokenize(&line) {
                Ok(tokens) => tokens,
                Err(e) => {
                    eprintln!("syntax error: {:?}", e);
                    continue;
                }
            }
        };

        if tokens.is_empty() {
            continue;
        }

        let token_refs: Vec<&str> = tokens.iter().map(|s| s.as_str()).collect();
        let command = token_refs[0];

        let _ = rl.add_history_entry(line.as_str());
        let (args, ctx) = IoContext::parse_context(&&token_refs[1..]);

        match command {
            "echo" => shell.builtin_echo(&args, &ctx),
            "type" => shell.builtin_type(&args, &ctx),
            "cd" => shell.builtin_cd(&args, &ctx),
            "pwd" => shell.builtin_pwd(&ctx),
            "declare" => shell.builtin_declare(&args, &ctx),
            "history" => shell.builtin_history(&args, &ctx, &mut rl),
            "complete" => {
                if let Some(helper_ref) = rl.helper() {
                    shell.builtin_complete(&args, &ctx, helper_ref);
                }
            }
            "exit" => {
                save_histfile_env(&rl);
                break;
            }

            _ => shell.execute(command, &args, &ctx),
        }
    }
}

fn save_histfile_env(rl: &Editor<ShellHelper, DefaultHistory>) {
    let Some(hist_env) = env::var_os("HISTFILE") else {
        return;
    };

    if let Ok(mut f) = File::create(hist_env) {
        rl.history().iter().for_each(|line| writeln!(f, "{}", line).unwrap());
    }
}
