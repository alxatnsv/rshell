pub struct CompletionContext<'a> {
    pub line: &'a str,        // -> COMP_LINE
    pub pos: usize,           // -> COMP_POINT
    pub start_pos: usize,
    pub current_word: &'a str, // full partial word, e.g. "src/Carg"
    pub dir: Option<&'a str>,  // "src"      (path completion only)
    pub file: &'a str,         // "Carg"     (path / command completion only)
    pub words: Vec<&'a str>,
    pub ends_with_space: bool,
}

impl<'a> CompletionContext<'a> {
    pub fn parse(line: &'a str, pos: usize) -> Self {
        let before_cursor = &line[..pos];
        let start_pos = before_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let current_word = &before_cursor[start_pos..];

        let (dir, file) = match current_word.rsplit_once('/') {
            Some((d, f)) => (Some(d), f),
            None => (None, current_word),
        };

        Self {
            line,
            pos,
            start_pos,
            current_word,
            dir,
            file,
            words: before_cursor.split_whitespace().collect(),
            ends_with_space: before_cursor.ends_with(char::is_whitespace),
        }
    }
    
    pub fn is_command_position(&self) -> bool {
        self.words.len() <= 1 && !self.ends_with_space
    }

    pub fn command(&self) -> Option<&str> {
        self.words.first().copied()
    }

    pub fn previous_word(&self) -> Option<&str> {
        // word before the one being completed
        let n = self.words.len();
        if self.ends_with_space {
            self.words.last().copied()
        } else if n >= 2 {
            Some(self.words[n - 2])
        } else {
            None
        }
    }
}