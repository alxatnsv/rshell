use crate::{
    helper::ShellHelper,
    io_context::IoContext,
    streams::{Input, Output, Streams},
};
use is_executable::IsExecutable;
use rustyline::{Editor, history::DefaultHistory};
use std::io::Write;
use std::{
    env,
    io::{PipeReader, pipe},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Child, Command},
};

use super::Shell;

impl Shell {
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
            self.add_job(children, line.to_string());
        } else {
            for mut child in children {
                let _ = child.wait();
            }
        }
    }

    pub(super) fn spawn_external(&self, command: &str, args: &[&str], io: Streams) -> Option<Child> {
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

    pub(super) fn locate_executable(command: &str) -> Option<PathBuf> {
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
}
