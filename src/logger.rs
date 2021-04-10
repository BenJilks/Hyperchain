use std::io::Write;

#[derive(PartialEq, PartialOrd, Debug)]
pub enum LoggerLevel
{
    Verbose = 0,
    Info,
    Warning,
    Error,
}

pub struct Logger<W>
    where W: Write
{
    stream: W,
    log_level: LoggerLevel,
}

impl<W> Logger<W>
    where W: Write
{

    pub fn new(stream: W, log_level: LoggerLevel) -> Self
    {
        Self
        {
            stream,
            log_level,
        }
    }

    pub fn log(&mut self, level: LoggerLevel, msg: &str)
    {
        if level < self.log_level {
            return;
        }

        let full_message = format!("{:?}: {}\n", level, msg);
        self.stream.write(full_message.as_bytes()).unwrap();
        self.stream.flush().unwrap();
    }

}
