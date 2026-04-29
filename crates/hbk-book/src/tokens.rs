use std::fmt;

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

pub(crate) fn tokenize(content: &str) -> Vec<String> {
    const BOM: char = '\u{feff}';
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = content.chars().peekable();
    let mut in_string = false;

    while let Some(ch) = chars.next() {
        match ch {
            BOM => {}
            '"' if in_string => {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    current.push(ch);
                    tokens.push(std::mem::take(&mut current));
                    in_string = false;
                }
            }
            '"' => {
                push_token(&mut tokens, &mut current);
                current.push(ch);
                in_string = true;
            }
            _ if in_string => current.push(ch),
            ch if ch.is_whitespace() => push_token(&mut tokens, &mut current),
            '{' | '}' | ',' => {
                push_token(&mut tokens, &mut current);
                if ch != ',' {
                    tokens.push(ch.to_string());
                }
            }
            _ => current.push(ch),
        }
    }
    push_token(&mut tokens, &mut current);
    tokens
}

fn push_token(tokens: &mut Vec<String>, current: &mut String) {
    let token = current.trim();
    if !token.is_empty() {
        tokens.push(token.to_string());
    }
    current.clear();
}

pub(crate) struct TokenParser {
    tokens: Vec<String>,
    index: usize,
}

impl TokenParser {
    pub(crate) fn new(tokens: Vec<String>) -> Self {
        Self { tokens, index: 0 }
    }

    pub(crate) fn peek(&self) -> Option<&str> {
        self.tokens.get(self.index).map(String::as_str)
    }

    pub(crate) fn next(&mut self, context: impl AsRef<str>) -> Result<String, TokenError> {
        let token = self.tokens.get(self.index).cloned().ok_or_else(|| {
            TokenError::new(format!("{}: unexpected end of input", context.as_ref()))
        })?;
        self.index += 1;
        Ok(token)
    }

    pub(crate) fn expect(
        &mut self,
        expected: &str,
        context: impl AsRef<str>,
    ) -> Result<(), TokenError> {
        let token = self.next(context.as_ref())?;
        if token != expected {
            return Err(TokenError::new(format!(
                "{}: expected '{expected}', got '{token}'",
                context.as_ref()
            )));
        }
        Ok(())
    }

    pub(crate) fn number(&mut self, context: impl AsRef<str>) -> Result<usize, TokenError> {
        let token = self.next(context.as_ref())?;
        token.parse::<usize>().map_err(|source| {
            TokenError::new(format!(
                "{}: expected number, got '{token}': {source}",
                context.as_ref()
            ))
        })
    }

    pub(crate) fn string(&mut self, context: impl AsRef<str>) -> Result<String, TokenError> {
        let token = self.next(context.as_ref())?;
        if !token.starts_with('"') || !token.ends_with('"') {
            return Err(TokenError::new(format!(
                "{}: expected string, got '{token}'",
                context.as_ref()
            )));
        }
        Ok(token[1..token.len() - 1].to_string())
    }

    pub(crate) fn expect_end(&self, context: &str) -> Result<(), TokenError> {
        if self.index == self.tokens.len() {
            Ok(())
        } else {
            Err(TokenError::new(format!(
                "{context}: unexpected trailing token '{}'",
                self.tokens[self.index]
            )))
        }
    }
}
