use crate::{helper::ShellHelper, io_context::IoContext, job::Job, streams::{Input, Output, Streams}};
use is_executable::IsExecutable;
use rustyline::{
    Editor,
    history::{DefaultHistory, History},
};

#[allow(unused_imports)]
use std::io::{self, Write};
use std::{
    cell::{Cell, Ref, RefCell}, collections::HashMap, env, fs::{File, OpenOptions}, io::{PipeReader, pipe}, os::unix::process::CommandExt, path::PathBuf, process::{Child, Command},
};

#[derive(Debug)]
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

    pub fn run_pipeline(&self, stages: &[Vec<&str>], background: bool, line: &str, rl: &mut Editor<ShellHelper, DefaultHistory>) {
        let last = stages.len() - 1;
        let mut prev_reader: Option<PipeReader> = None;
        let mut children: Vec<Child> = Vec::new();

        for (i, stage) in stages.iter().enumerate() {
            let command = stage[0];
            let (args, ctx) = IoContext::parse_context(&stage[1..]);

            let mut io = match Streams::from_context(&ctx) {
                Ok(io) => io,
                Err(e) => {
                    eprintln!("{e}");
                    break;
                }
            };

            if let Some(reader) = prev_reader.take() {
                io.input = Input::Pipe(reader);
            }

            if i < last {
                match pipe() {
                    Ok((reader, writer)) => {
                        if ctx.stdout_redirect.is_none() {
                            io.out = Output::Pipe(writer);
                        }
                        prev_reader = Some(reader);
                    }
                    Err(e) => {
                        eprintln!("pipe: {e}");
                        break;
                    }
                }
            }

            if let Some(child) = self.run_stage(command, &args, io, rl) {
                children.push(child);
            }
        }

        if background {
            if let Some(last_child) = children.pop() {
                for mut c in children {
                    let _ = c.wait();
                }
                self.add_job(last_child, line.to_string());
            }
        } else {
            for mut child in children {
                let _ = child.wait();
            }
        }
    }

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

    fn spawn_external(&self, command: &str, args: &[&str], io: Streams) -> Option<Child> {
        let Some(path) = Self::locate_executable(command) else {
            let mut io = io;
            let _ = writeln!(io.err, "{command}: not found");
            return None;
        };

        let result = Command::new(path).arg0(command).args(args).stdin(io.input).stdout(io.out).stderr(io.err).spawn();

        match result {
            Ok(child) => Some(child),
            Err(e) => {
                eprintln!("{command}: {e}");
                None
            }
        }
    }

    pub fn add_job(&self, child: Child, command_line: String) {
        let job = Job::new(self.jobs.borrow().len() + 1, child, command_line);
        println!("[{}] {}", job.number, job.child.id());
        self.jobs.borrow_mut().push(job);
    }

    pub fn get_variables(&self) -> Ref<'_, HashMap<String, String>> {
        self.variables.borrow()
    }

    pub fn builtin_echo(&self, args: &[&str], io: &mut Streams) {
        let _ = writeln!(io.out, "{}", args.join(" "));
    }

    pub fn builtin_complete(&self, args: &[&str], io: &mut Streams, helper: &ShellHelper) {
        let Some(param) = args.get(0) else {
            return;
        };

        match param {
            &"-p" => {
                if let Some(cmd) = args.get(1) {
                    let compls = helper.get_completion_spec();
                    let Some(path) = compls.get(*cmd) else {
                        let _ = writeln!(io.err, "complete: {cmd}: no completion specification");
                        return;
                    };
                    let _ = writeln!(io.out, "complete -C '{path}' {cmd}");
                }
            }
            &"-C" => {
                if let (Some(path), Some(cmd)) = (args.get(1), args.get(2)) {
                    helper.set_completion_spec(cmd, path);
                }
            }
            &"-r" => {
                if let Some(cmd) = args.get(1) {
                    helper.remove_completion_spec(cmd);
                }
            }
            _ => {}
        }
    }

    pub fn builtin_jobs(&self, io: &mut Streams) {
        let mut jobs = self.jobs.borrow_mut();
        let len = jobs.len();

        for (idx, job) in jobs.iter_mut().enumerate() {
            let sign = if idx + 1 == len {
                '+'
            } else if idx + 2 == len {
                '-'
            } else {
                ' '
            };

            let status_str: &str;
            let command;
            match job.child.try_wait() {
                Ok(Some(_)) => {
                    status_str = "Done";
                    job.running = false;
                    command = job.command_line.clone();
                }
                Ok(None) => {
                    status_str = "Running";
                    command = job.command_line.to_owned() + " &";
                }
                Err(_) => {
                    status_str = "Error";
                    command = job.command_line.clone();
                }
            };
            let _ = writeln!(io.out, "[{}]{}  {:<24}{}", job.number, sign, status_str, command);
        }

        jobs.retain(|x| x.running);
    }

    pub fn builtin_history(&self, args: &[&str], io: &mut Streams, rl: &mut Editor<ShellHelper, DefaultHistory>) {
        let len = rl.history().len();
        let mut limit = len;

        if let Some(input_num) = args.get(0).and_then(|arg| arg.parse::<usize>().ok()) {
            limit = input_num.min(len);
        }

        if let Some(file_path) = args.get(1).map(PathBuf::from) {
            let action = args[0];
            self.history_action(&action, &file_path, io, rl);
            return;
        }

        let start_idx = len - limit;

        for (i, entry) in rl.history().iter().skip(start_idx).enumerate() {
            let _ = writeln!(io.out, "{:>5}  {}", start_idx + i + 1, entry);
        }
    }

    pub fn builtin_declare(&self, args: &[&str], io: &mut Streams) {
        let Some(param) = args.get(0) else {
            return;
        };

        match param {
            &"-p" => {
                if let Some(key) = args.get(1) {
                    let vars = self.variables.borrow();
                    let Some(value) = vars.get(*key) else {
                        let _ = writeln!(io.err, "declare: {key}: not found");
                        return;
                    };
                    let _ = writeln!(io.out, "declare -- {key}=\"{value}\"");
                }
            }
            _ => {
                if let Some((key, value)) = param.split_once("=") {
                    let true = self.is_valid_var_identifier(key) else {
                        let _ = writeln!(io.err, "declare: `{key}={value}': not a valid identifier");
                        return;
                    };
                    self.set_variable(key, value);
                }
            }
        }
    }

    pub fn builtin_type(&self, args: &[&str], io: &mut Streams) {
        let Some(&target) = args.first() else { return };

        if self.is_builtin(target) {
            let _ = writeln!(io.out, "{target} is a shell builtin");
            return;
        }

        match Shell::locate_executable(target) {
            Some(path) => {
                let _ = writeln!(io.out, "{target} is {}", path.display());
            }
            None => {
                let _ = writeln!(io.err, "{target}: not found");
            }
        }
    }

    pub fn builtin_pwd(&self, io: &mut Streams) {
        let _ = writeln!(io.out, "{}", env::current_dir().unwrap_or_default().display());
    }

    pub fn builtin_cd(&self, args: &[&str], io: &mut Streams) {
        let arg = args.first().copied().unwrap_or("");

        let home_path = if arg != "~" { PathBuf::from(arg) } else { self.get_home_path() };

        if let Err(_) = env::set_current_dir(&home_path) {
            let _ = writeln!(io.err, "cd: {}: No such file or directory", home_path.display());
        }
    }

    // pub fn execute(&self, command: &str, args: &[&str], ctx: &IoContext) {
    //     let Some(path_buf) = Shell::locate_executable(command) else {
    //         self.error(&ctx, &format!("{command}: not found"));
    //         return;
    //     };

    //     let mut cmd = Command::new(path_buf);
    //     cmd.arg0(command).args(args);

    //     if let Some(file_path) = &ctx.stdout_redirect {
    //         if let Ok(file) = self.open_or_create(ctx.append, file_path) {
    //             cmd.stdout(file);
    //         }
    //     }

    //     if let Some(file_path) = &ctx.stderr_redirect {
    //         if let Ok(file) = self.open_or_create(ctx.append, file_path) {
    //             cmd.stderr(file);
    //         }
    //     }

    //     if ctx.background {
    //         if let Ok(child) = cmd.spawn() {
    //             let command_line = if args.is_empty() { format!("{command}") } else { format!("{command} {}", args.join(" ")) };
    //             let job = Job::new(self.jobs.borrow().len() + 1, child, command_line);
    //             println!("[{}] {}", job.number, job.child.id());
    //             self.jobs.borrow_mut().push(job);
    //         }
    //     } else {
    //         let _ = cmd.status();
    //     }
    // }

    pub fn check_jobs(&self) {
        let mut jobs = self.jobs.borrow_mut();

        for job in jobs.iter_mut() {
            if job.child.try_wait().is_ok_and(|status| status.is_some()) {
                job.running = false;
            }
        }

        let len = jobs.iter().filter(|x| !x.running).count();

        for (idx, job) in jobs.iter().filter(|x| !x.running).enumerate() {
            let sign = if idx + 1 == len {
                '+'
            } else if idx + 2 == len {
                '-'
            } else {
                ' '
            };

            println!("[{}]{}  {:<24}{}", job.number, sign, "Done", job.command_line);
        }

        jobs.retain(|x| x.running);
    }

    fn get_home_path(&self) -> PathBuf {
        let Some(home) = env::var_os("HOME") else {
            return PathBuf::new();
        };

        PathBuf::from(home)
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

    // fn output(&self, ctx: &IoContext, args: &str) {
    //     if let Some(redirect_path) = &ctx.stderr_redirect {
    //         let _ = File::create(redirect_path);
    //     }

    //     if let Some(redirect_path) = &ctx.stdout_redirect {
    //         if let Ok(mut f) = self.open_or_create(ctx.append, redirect_path) {
    //             let _ = writeln!(f, "{args}");
    //         }
    //     } else {
    //         println!("{args}");
    //     }
    // }

    // fn error(&self, ctx: &IoContext, args: &str) {
    //     if let Some(redirect_path) = &ctx.stdout_redirect {
    //         let _ = File::create(redirect_path);
    //     }

    //     if let Some(redirect_path) = &ctx.stderr_redirect {
    //         if let Ok(mut f) = self.open_or_create(ctx.append, redirect_path) {
    //             let _ = writeln!(f, "{args}");
    //         }
    //     } else {
    //         eprintln!("{args}");
    //     }
    // }

    fn open_or_create(&self, append: bool, path: &PathBuf) -> Result<File, std::io::Error> {
        if append { OpenOptions::new().create(true).append(true).open(path) } else { File::create(path) }
    }

    fn history_action(&self, action: &str, file_path: &PathBuf, io: &mut Streams, rl: &mut Editor<ShellHelper, DefaultHistory>) {
        match action {
            "-r" => {
                let _ = rl.load_history(&file_path);
                self.history_sync_idx.set(rl.history().len());
                return;
            }
            "-w" => {
                if let Ok(mut f) = self.open_or_create(false, &file_path) {
                    rl.history().iter().for_each(|line| writeln!(f, "{}", line).unwrap());
                }

                self.history_sync_idx.set(rl.history().len());
                return;
            }
            "-a" => {
                let sync_idx = self.history_sync_idx.get();
                if sync_idx < rl.history().len() {
                    if let Ok(mut f) = self.open_or_create(true, &file_path) {
                        for line in rl.history().iter().skip(sync_idx) {
                            writeln!(f, "{}", line).unwrap();
                        }
                    }

                    self.history_sync_idx.set(rl.history().len());
                }
                return;
            }
            _ => {
                let _ = writeln!(io.err, "{action}: invalid option");
                return;
            }
        }
    }

    fn set_variable(&self, key: &str, value: &str) {
        self.variables.borrow_mut().insert(key.to_string(), value.to_string());
    }

    fn is_valid_var_identifier(&self, name: &str) -> bool {
        let Some((&first, rest)) = name.as_bytes().split_first() else {
            return false;
        };

        (first.is_ascii_alphabetic() || first == b'_') && rest.iter().all(|&b| b.is_ascii_alphanumeric() || b == b'_')
    }
}
