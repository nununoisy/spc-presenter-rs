

pub struct Tokenizer<'a> {
    inner: &'a str,
    pos: usize
}

impl<'a> Tokenizer<'a> {
    pub(crate) fn new(inner: &'a str) -> Self {
        Self {
            inner,
            pos: 0
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.inner.len() {
            return None;
        }

        let result_start = self.pos;
        let mut result_end: Option<usize> = None;
        let mut is_path = false;
        let mut escaped = false;

        while let Some(c) = self.inner.chars().nth(self.pos) {
            if result_end.is_some() && !is_path && !c.is_whitespace() {
                break;
            } else if c == '\\' && !escaped {
                escaped = true;
                self.pos += 1;
                continue;
            } else if c == '"' && !escaped {
                if is_path {
                    result_end = Some((self.pos + 1).min(self.inner.len()));
                    is_path = false;
                } else if result_end.is_none() {
                    is_path = true;
                }
            } else if result_end.is_none() && c.is_whitespace() && !is_path && !escaped {
                result_end = Some(self.pos);
            }

            escaped = false;
            self.pos += 1;
        }

        let result_end = result_end.unwrap_or(self.pos);
        Some(&self.inner[result_start..result_end])
    }
}

pub trait TokenizerAdapter<'a> {
    fn tokenize(&self) -> Tokenizer<'a>;
}

impl<'a> TokenizerAdapter<'a> for &'a str {
    fn tokenize(&self) -> Tokenizer<'a> {
        Tokenizer::new(self)
    }
}
