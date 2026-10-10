mod completion_context;
mod helper;
mod io_context;
mod job;
mod lexer;
mod pipeline;
mod shell;
mod streams;

use helper::ShellHelper;
use lexer::Lexer;
use rustyline::{Config, Editor, history::DefaultHistory};
use shell::Shell;
use std::io::Write;
use std::{env, fs::File};

fn main() {
    let shell = Shell::new();

    let config = Config::builder().history_ignore_dups(false).unwrap().completion_type(rustyline::CompletionType::List).build();
    let mut rl = Editor::<ShellHelper, DefaultHistory>::with_config(config).unwrap();
    rl.set_helper(Some(ShellHelper::new()));
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

        let _ = rl.add_history_entry(line.as_str());

        let mut token_refs: Vec<&str> = tokens.iter().map(|s| s.as_str()).collect();

        let background = token_refs.last() == Some(&"&");
        if background {
            token_refs.pop();
        }

        let stages = match pipeline::split_pipeline(&token_refs) {
            Ok(stages) => stages,
            Err(e) => {
                eprintln!("syntax error: {:?}", e);
                continue;
            }
        };

        if stages.len() == 1 && stages[0][0] == "exit" {
            save_histfile_env(&rl);
            break;
        }

        shell.run_pipeline(&stages, background, &token_refs.join(" "), &mut rl);
        shell.check_jobs();
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
