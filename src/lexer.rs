use std::{collections::HashMap, mem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeContext {
    Normal,
    DoubleQuote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Normal,
    InSingleQuote,
    InDoubleQuote,
    Escape(EscapeContext),
    SawDollar(EscapeContext),
    ReadingVar(EscapeContext),
    ReadingBracedVar(EscapeContext),
}

#[derive(Debug, PartialEq, Eq)]
pub enum LexError {
    UnclosedQuote(char),
    UnclosedBrace,
    TrailingEscape,
}

pub struct Lexer<'a> {
    state: State,
    tokens: Vec<String>,
    current_word: String,
    var_buffer: String,
    in_word: bool,
    variables: &'a HashMap<String, String>,
}

impl<'a> Lexer<'a> {
    pub fn new(variables: &'a HashMap<String, String>) -> Self {
        Self {
            state: State::Normal,
            tokens: Vec::new(),
            current_word: String::new(),
            var_buffer: String::new(),
            in_word: false,
            variables,
        }
    }

    pub fn tokenize(mut self, input: &str) -> Result<Vec<String>, LexError> {
        for ch in input.chars() {
            self.step(ch);
        }

        self.finish()
    }

    fn step(&mut self, ch: char) {
        match self.state {
            State::Normal => match ch {
                ' ' | '\t' | '\n' => {
                    self.flush_word();
                }
                '\'' => {
                    self.in_word = true;
                    self.state = State::InSingleQuote;
                }
                '"' => {
                    self.in_word = true;
                    self.state = State::InDoubleQuote;
                }
                '\\' => {
                    self.in_word = true;
                    self.state = State::Escape(EscapeContext::Normal);
                }
                '$' => {
                    self.state = State::SawDollar(EscapeContext::Normal);
                }
                _ => {
                    self.in_word = true;
                    self.current_word.push(ch);
                }
            },

            State::InSingleQuote => match ch {
                '\'' => {
                    self.state = State::Normal;
                }
                _ => {
                    self.current_word.push(ch);
                }
            },

            State::InDoubleQuote => match ch {
                '"' => {
                    self.state = State::Normal;
                }
                '\\' => {
                    self.state = State::Escape(EscapeContext::DoubleQuote);
                }
                '$' => {
                    self.state = State::SawDollar(EscapeContext::DoubleQuote);
                }
                _ => {
                    self.current_word.push(ch);
                }
            },

            State::Escape(EscapeContext::Normal) => {
                self.current_word.push(ch);
                self.state = State::Normal;
            }

            State::Escape(EscapeContext::DoubleQuote) => {
                if Self::is_escapable_in_double_quotes(ch) {
                    self.current_word.push(ch);
                } else {
                    self.current_word.push('\\');
                    self.current_word.push(ch);
                }
                self.state = State::InDoubleQuote;
            }

            State::SawDollar(return_to) => match ch {
                '{' => {
                    self.var_buffer.clear();
                    self.state = State::ReadingBracedVar(return_to);
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    self.var_buffer.clear();
                    self.var_buffer.push(ch);
                    self.state = State::ReadingVar(return_to);
                }
                _ => {
                    self.in_word = true;
                    self.current_word.push('$');
                    self.state = Self::context_to_state(return_to);
                    self.step(ch);
                }
            },

            State::ReadingVar(return_to) => {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    self.var_buffer.push(ch);
                } else {
                    let val = self.get_variable(&self.var_buffer);
                    if !val.is_empty() {
                        self.in_word = true;
                        self.current_word.push_str(&val);
                    }
                    self.state = Self::context_to_state(return_to);
                    self.step(ch);
                }
            }

            State::ReadingBracedVar(return_to) => {
                if ch == '}' {
                    let val = self.get_variable(&self.var_buffer);
                    if !val.is_empty() {
                        self.in_word = true;
                        self.current_word.push_str(&val);
                    }
                    self.state = Self::context_to_state(return_to);
                } else {
                    self.var_buffer.push(ch);
                }
            }
        }
    }

    fn get_variable(&self, name: &str) -> String {
        if let Some(val) = self.variables.get(name) { val.clone() } else { String::new() }
    }

    fn context_to_state(ctx: EscapeContext) -> State {
        match ctx {
            EscapeContext::Normal => State::Normal,
            EscapeContext::DoubleQuote => State::InDoubleQuote,
        }
    }

    fn flush_word(&mut self) {
        if self.in_word {
            self.tokens.push(mem::take(&mut self.current_word));
            self.in_word = false;
        }
    }

    fn is_escapable_in_double_quotes(ch: char) -> bool {
        matches!(ch, '$' | '`' | '"' | '\\' | '\n')
    }

    fn finish(mut self) -> Result<Vec<String>, LexError> {
        match self.state {
            State::Normal => {
                self.flush_word();
                Ok(self.tokens)
            }
            State::SawDollar(_) => {
                self.in_word = true;
                self.current_word.push('$');
                self.flush_word();
                Ok(self.tokens)
            }
            State::ReadingVar(_) => {
                let val = self.get_variable(&self.var_buffer);
                if !val.is_empty() {
                    self.in_word = true;
                    self.current_word.push_str(&val);
                }
                self.flush_word();
                Ok(self.tokens)
            }
            State::InSingleQuote => Err(LexError::UnclosedQuote('\'')),
            State::InDoubleQuote => Err(LexError::UnclosedQuote('"')),
            State::Escape(_) => Err(LexError::TrailingEscape),
            State::ReadingBracedVar(_) => Err(LexError::UnclosedBrace),
        }
    }
}
