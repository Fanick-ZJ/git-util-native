use std::{error::Error, fmt::{Display, Formatter}};

pub fn build_git_error(path: &str, message: &str) -> CustomerGitError {
    CustomerGitError {
        path: path.to_string(),
        message: message.to_string(),
        inner_error: None,
    }
}

#[derive(Debug)]
pub struct CustomerGitError {
    pub path: String,
    pub message: String,
    pub inner_error: Option<Box<dyn Error>>,
}

impl Error for CustomerGitError {
    fn description(&self) -> &str {
        &self.path
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.inner_error.as_deref()
    }
    
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl Display for CustomerGitError{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.inner_error {
            Some(ref err) => write!(f, "{}", err),
            None => write!(f, "The path:{} => {}", self.path, self.message),
        }
    }
}