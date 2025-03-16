use std::borrow::Cow;
use std::fmt::Display;

#[derive(Debug, Clone, Default)]
pub struct LogArgs<'a> {
    args: Vec<Cow<'a, str>>,
}

impl<'a> LogArgs<'a> {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    pub fn add_item_split(&mut self) -> &mut Self {
        self.args.push(Cow::Borrowed("|"));
        self
    }
    pub fn add_key_split(&mut self) -> &mut Self {
        self.args.push(Cow::Borrowed(":"));
        self
    }
    pub fn add_value_split(&mut self) -> &mut Self {
        self.args.push(Cow::Borrowed(";"));
        self
    }
    pub fn add_str(&mut self, arg: &'a str) -> &mut Self {
        self.args.push(Cow::Borrowed(arg));
        self
    }
    pub fn add_string(&mut self, arg: String) -> &mut Self {
        self.args.push(Cow::Owned(arg));
        self
    }

    pub fn append_args(&mut self, other_args: LogArgs<'a>) -> &mut Self {
        if other_args.args.is_empty() {
            return self;
        }
        self.add_string(other_args.to_string());
        self
    }

    pub fn merge_args(&mut self, mut other_args: LogArgs<'a>) -> &mut Self {
        if other_args.args.is_empty() {
            return self;
        }
        let b_first_is_split = match self.args.first() {
            Some(arg) => arg == &Cow::Borrowed("|"),
            None => false,
        };
        let self_last_is_split = match self.args.last() {
            Some(arg) => arg == &Cow::Borrowed("|"),
            None => false,
        };
        if !b_first_is_split && !self_last_is_split {
            self.args.push(Cow::Borrowed("|"));
        } else if self_last_is_split && b_first_is_split {
            other_args.args.remove(0);
        }
        self.add_string(other_args.to_string());
        self
    }
}

impl Display for LogArgs<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for arg in self.args.iter() {
            write!(f, "{}", arg)?;
        }
        Ok(())
    }
}
