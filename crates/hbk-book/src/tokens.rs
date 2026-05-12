use std::fmt;
use winnow::Parser;
use winnow::error::EmptyError;
use winnow::token::{literal, take_while};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenError {
    message: String,
}

impl TokenError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TokenError {}

const BOM: char = '\u{feff}';

pub(crate) struct TokenParser<'a> {
    input: &'a str,
}

impl<'a> TokenParser<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Self { input }
    }

    pub(crate) fn next_is(&mut self, expected: char) -> bool {
        self.skip_trivia();
        self.input.starts_with(expected)
    }

    pub(crate) fn has_more(&mut self) -> bool {
        self.skip_trivia();
        !self.input.is_empty()
    }

    pub(crate) fn expect(
        &mut self,
        expected: &str,
        context: impl AsRef<str>,
    ) -> Result<(), TokenError> {
        self.skip_trivia();
        if self.input.is_empty() {
            return Err(TokenError::new(format!(
                "{}: unexpected end of input",
                context.as_ref()
            )));
        }
        if literal::<_, _, EmptyError>(expected)
            .parse_next(&mut self.input)
            .is_err()
        {
            return Err(TokenError::new(format!(
                "{}: expected '{expected}', got '{}'",
                context.as_ref(),
                self.preview_token()
            )));
        }
        Ok(())
    }

    pub(crate) fn number(&mut self, context: impl AsRef<str>) -> Result<usize, TokenError> {
        self.skip_trivia();
        if self.input.is_empty() {
            return Err(TokenError::new(format!(
                "{}: unexpected end of input",
                context.as_ref()
            )));
        }
        let token = self.next_token();
        token.parse::<usize>().map_err(|source| {
            TokenError::new(format!(
                "{}: expected number, got '{token}': {source}",
                context.as_ref()
            ))
        })
    }

    pub(crate) fn string(&mut self, context: impl AsRef<str>) -> Result<String, TokenError> {
        self.skip_trivia();
        if self.input.is_empty() {
            return Err(TokenError::new(format!(
                "{}: unexpected end of input",
                context.as_ref()
            )));
        }
        if !self.input.starts_with('"') {
            return Err(TokenError::new(format!(
                "{}: expected string, got '{}'",
                context.as_ref(),
                self.preview_token()
            )));
        }
        self.input = &self.input[1..];
        let mut value = String::new();
        loop {
            let Some(ch) = self.input.chars().next() else {
                return Err(TokenError::new(format!(
                    "{}: unexpected end of input",
                    context.as_ref()
                )));
            };
            self.input = &self.input[ch.len_utf8()..];
            if ch == '"' {
                if self.input.starts_with('"') {
                    value.push('"');
                    self.input = &self.input[1..];
                } else {
                    return Ok(value);
                }
            } else if ch != BOM {
                value.push(ch);
            }
        }
    }

    pub(crate) fn expect_end(&mut self, context: &str) -> Result<(), TokenError> {
        self.skip_trivia();
        if self.input.is_empty() {
            Ok(())
        } else {
            Err(TokenError::new(format!(
                "{context}: unexpected trailing token '{}'",
                self.preview_token()
            )))
        }
    }

    fn skip_trivia(&mut self) {
        let _: Result<_, EmptyError> =
            take_while(0.., |ch: char| ch == BOM || ch == ',' || ch.is_whitespace())
                .parse_next(&mut self.input);
    }

    fn next_token(&mut self) -> String {
        let Some(first) = self.input.chars().next() else {
            return "<end>".to_string();
        };
        if first == '{' || first == '}' {
            self.input = &self.input[first.len_utf8()..];
            return first.to_string();
        }
        let end = self
            .input
            .char_indices()
            .find_map(|(index, ch)| {
                (ch == BOM || ch == ',' || ch.is_whitespace() || ch == '{' || ch == '}')
                    .then_some(index)
            })
            .unwrap_or(self.input.len());
        let token = self.input[..end].to_string();
        self.input = &self.input[end..];
        token
    }

    fn preview_token(&self) -> String {
        let mut chars = self.input.chars();
        match chars.next() {
            None => "<end>".to_string(),
            Some(ch) if ch == '{' || ch == '}' => ch.to_string(),
            Some('"') => {
                let mut token = String::from("\"");
                for ch in chars {
                    token.push(ch);
                    if ch == '"' {
                        break;
                    }
                }
                token
            }
            Some(first) => {
                let mut token = String::new();
                token.push(first);
                for ch in chars {
                    if ch == BOM || ch == ',' || ch.is_whitespace() || ch == '{' || ch == '}' {
                        break;
                    }
                    token.push(ch);
                }
                token
            }
        }
    }
}
