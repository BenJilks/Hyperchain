use super::packet_handler::{Packet, Message};
use super::{TcpSender, TcpReceiver};

use tcp_channel::{ReceiverBuilder, ChannelRecv};
use tcp_channel::{SenderBuilder, ChannelSend};
use tcp_channel::LittleEndian;
use std::io::{BufReader, BufWriter};
use std::net::TcpStream;
use std::thread::JoinHandle;
use std::sync::mpsc::Sender;

pub struct Connection
{
    stream: TcpStream,
    reciver_thread: Option<JoinHandle<()>>,
    pub sender: TcpSender<Packet>,
    pub public_address: Option<String>,
}

impl Connection
{

    pub fn new(port: u16, address: &str, stream: TcpStream, 
               message_sender: Sender<Message>) 
        -> std::io::Result<Self>
    {
        let reciver = ReceiverBuilder::new()
            .with_type::<Packet>()
            .with_endianness::<LittleEndian>()
            .build(BufReader::new(stream.try_clone()?));
        let reciver_thread = start_packet_reciver(address.to_owned(), reciver, message_sender);

        let mut sender = SenderBuilder::new()
            .with_type::<Packet>()
            .with_endianness::<LittleEndian>()
            .build(BufWriter::new(stream.try_clone()?));
        if sender.send(&Packet::OnConnected(port)).is_err() {
            return Err(std::io::Error::from(std::io::ErrorKind::NotConnected));
        }
        sender.flush()?;

        Ok(Self
        {
            stream,
            reciver_thread: Some( reciver_thread ),
            sender,
            public_address: None,
        })
    }

}

impl Drop for Connection
{

    fn drop(&mut self)
    {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
        self.reciver_thread
            .take().unwrap()
            .join().expect("Join server connection");
    }

}

pub fn start_packet_reciver(server_ip: String, mut recv: TcpReceiver<Packet>, 
                            message_sender: Sender<Message>)
    -> JoinHandle<()>
{
    std::thread::spawn(move || loop
    {
        match recv.recv()
        {
            Ok(packet) =>
            {
                match message_sender.send(Message::Packet(server_ip.clone(), packet)) 
                {
                    Ok(_) => {},
                    Err(err) => 
                    {
                        error!("message_sender.send(packet): {}", err);
                        break;
                    },
                }
            },

            Err(tcp_channel::RecvError::IoError(e)) 
                if e.kind() == std::io::ErrorKind::UnexpectedEof ||
                   e.kind() == std::io::ErrorKind::ConnectionReset =>
            {
                // The stream has closed
                break;
            },
            
            Err(err) =>
            {
                error!("recv.recv(): {}", err);
                break;
            },
        }
    })
}
