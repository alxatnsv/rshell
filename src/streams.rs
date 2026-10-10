use std::fs::{File, OpenOptions};
use std::io::{self, PipeReader, PipeWriter, Write};
use std::path::Path;
use std::process::Stdio;

use crate::io_context::IoContext;

pub enum Input {
    Terminal,
    Pipe(PipeReader),
}

pub enum Output {
    Stdout,
    Stderr,
    File(File),
    Pipe(PipeWriter),
}

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Output::Stdout => io::stdout().write(buf),
            Output::Stderr => io::stderr().write(buf),
            Output::File(f) => f.write(buf),
            Output::Pipe(p) => p.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Output::Stdout => io::stdout().flush(),
            Output::Stderr => io::stderr().flush(),
            Output::File(f) => f.flush(),
            Output::Pipe(p) => p.flush(),
        }
    }
}

impl From<Output> for Stdio {
    fn from(o: Output) -> Stdio {
        match o {
            Output::Stdout | Output::Stderr => Stdio::inherit(),
            Output::File(f) => Stdio::from(f),
            Output::Pipe(p) => Stdio::from(p),
        }
    }
}

impl From<Input> for Stdio {
    fn from(i: Input) -> Stdio {
        match i {
            Input::Terminal => Stdio::inherit(),
            Input::Pipe(r) => Stdio::from(r),
        }
    }
}

pub struct Streams {
    pub input: Input,
    pub out: Output,
    pub err: Output,
}

impl Streams {
    pub fn from_context(ctx: &IoContext) -> io::Result<Self> {
        let out = match &ctx.stdout_redirect {
            Some(path) => Output::File(open(path, ctx.append)?),
            None => Output::Stdout,
        };
        let err = match &ctx.stderr_redirect {
            Some(path) => Output::File(open(path, ctx.append)?),
            None => Output::Stderr,
        };
        Ok(Self { input: Input::Terminal, out, err })
    }
}

fn open(path: &Path, append: bool) -> io::Result<File> {
    if append {
        OpenOptions::new().create(true).append(true).open(path)
    } else {
        File::create(path)
    }
}