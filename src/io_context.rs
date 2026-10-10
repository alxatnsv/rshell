#[allow(unused_imports)]
use std::io::{self, Write};
use std::path::PathBuf;
pub struct IoContext {
    pub stdout_redirect: Option<PathBuf>,
    pub stderr_redirect: Option<PathBuf>,
    pub append: bool,
}

impl IoContext {
    const REDIRECT_OPERATORS: &'static [&'static str] = &[">", "1>", "2>", ">>", "1>>", "2>>"];

    pub fn parse_context<'a>(args: &[&'a str]) -> (Vec<&'a str>, IoContext) {
        if let Some(op_pos) = args.iter().position(|&x| Self::is_redirect_op(x)) {
            if op_pos + 1 < args.len() {
                let redirect_op = args[op_pos];
                let file_path = PathBuf::from(args[op_pos + 1]);

                let (stdout_redirect, stderr_redirect) = match redirect_op {
                    ">" | "1>" | ">>" | "1>>" => (Some(file_path), None),
                    "2>" | "2>>" => (None, Some(file_path)),
                    _ => (None, None),
                };

                let append = match redirect_op {
                    ">>" | "1>>" | "2>>" => true,
                    _ => false,
                };

                return (
                    args[..op_pos].to_vec(),
                    IoContext {
                        stdout_redirect,
                        stderr_redirect,
                        append
                    },
                );
            }
        }

        (
            args.to_vec(),
            IoContext {
                stdout_redirect: None,
                stderr_redirect: None,
                append: false
            },
        )
    }

    fn is_redirect_op(s: &str) -> bool {
        Self::REDIRECT_OPERATORS.contains(&s)
    }
}
