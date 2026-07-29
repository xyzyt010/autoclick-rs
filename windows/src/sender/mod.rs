pub mod console;
pub mod postmessage;
pub mod screen;
pub mod sendinput;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    PostMessage,
    WriteConsoleInput,
    SendInput,
    ScreenInput,
}

impl Method {
    pub fn name(&self) -> &'static str {
        match self {
            Method::PostMessage => "PostMessage",
            Method::WriteConsoleInput => "WriteConsoleInput",
            Method::SendInput => "SendInput",
            Method::ScreenInput => "ScreenInput",
        }
    }
}
