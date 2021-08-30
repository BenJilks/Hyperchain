use crate::{Command, Response};

use tcp_channel;
use tcp_channel::{SenderBuilder, ChannelSend};
use tcp_channel::{ReceiverBuilder, ChannelRecv};
use tcp_channel::LittleEndian;
use std::net::TcpStream;
use std::error::Error;
use std::io::{BufReader, BufWriter};

pub struct Client
{
    sender: tcp_channel::Sender<Command, LittleEndian>,
    receiver: tcp_channel::Receiver<Response, LittleEndian>,
}

impl Client
{
    
    pub fn new() -> Result<Self, Box<dyn Error>>
    {
        // FIXME: Allow changing this
        let stream = TcpStream::connect("127.0.0.1:9988")?;

        let sender = SenderBuilder::new()
            .with_type::<Command>()
            .with_endianness::<LittleEndian>()
            .build(BufWriter::new(stream.try_clone()?));

        let receiver = ReceiverBuilder::new()
            .with_type::<Response>()
            .with_endianness::<LittleEndian>()
            .build(BufReader::new(stream.try_clone()?));

        Ok(Self
        {
            sender,
            receiver,
        })
    }

    pub fn send(&mut self, command: Command) -> Result<Response, Box<dyn Error>>
    {
        self.sender.send(&command)?;
        self.sender.flush()?;
        Ok(self.receiver.recv()?)
    }

}
