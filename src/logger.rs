use std::io::Write;

#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub enum LoggerLevel
{
    Verbose = 0,
    Info,
    Warning,
    Error,
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct StdLoggerOutput;

impl StdLoggerOutput
{

    pub fn new() -> Self { Self {} }

}

impl Write for StdLoggerOutput
{

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>
    {
        std::io::stdout().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()>
    {
        std::io::stdout().flush()
    }

}
