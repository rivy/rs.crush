use crate::lexer::Lexer;

#[derive(Debug)]
pub struct JobError {
    pub message: String,
}

pub fn parse_error(message: &str, _lexer: &Lexer) -> JobError {
    return JobError {
        message: String::from(message),
    };
}

pub fn argument_error(message: &str) -> JobError {
    return JobError {
        message: String::from(message),
    };
}

pub fn error(message: &str) -> JobError {
    return JobError {
        message: String::from(message),
    };
}
