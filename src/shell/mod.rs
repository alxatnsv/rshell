mod builtins;
mod executor;
mod jobs;

use crate::{helper::ShellHelper, job::Job, streams::Streams};
use rustyline::{Editor, history::DefaultHistory};
use std::{
    cell::{Cell, Ref, RefCell},
    collections::HashMap,
    env,
    fs::{File, OpenOptions},
    path::PathBuf,
    process::Child,
};

pub struct Shell {
    history_sync_idx: Cell<usize>,
    variables: RefCell<HashMap<String, String>>,
    jobs: RefCell<Vec<Job>>,
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            history_sync_idx: Cell::new(0),
            variables: RefCell::new(HashMap::new()),
            jobs: RefCell::new(Vec::new()),
        }
    }

    pub const BUILTINS: &[&str] = &["echo", "exit", "type", "pwd", "cd", "history", "declare", "complete", "jobs"];

    pub fn run_stage(&self, command: &str, args: &[&str], mut io: Streams, rl: &mut Editor<ShellHelper, DefaultHistory>) -> Option<Child> {
        match command {
            "echo" => self.builtin_echo(args, &mut io),
            "type" => self.builtin_type(args, &mut io),
            "cd" => self.builtin_cd(args, &mut io),
            "pwd" => self.builtin_pwd(&mut io),
            "declare" => self.builtin_declare(args, &mut io),
            "jobs" => self.builtin_jobs(&mut io),
            "history" => self.builtin_history(args, &mut io, rl),
            "complete" => {
                if let Some(helper) = rl.helper() {
                    self.builtin_complete(args, &mut io, helper);
                }
            }
            _ => return self.spawn_external(command, args, io),
        }
        None
    }

    fn open_or_create(&self, append: bool, path: &PathBuf) -> Result<File, std::io::Error> {
        if append { OpenOptions::new().create(true).append(true).open(path) } else { File::create(path) }
    }

    fn get_home_path(&self) -> PathBuf {
        let Some(home) = env::var_os("HOME") else {
            return PathBuf::new();
        };

        PathBuf::from(home)
    }

    pub fn get_variables(&self) -> Ref<'_, HashMap<String, String>> {
        self.variables.borrow()
    }
}
