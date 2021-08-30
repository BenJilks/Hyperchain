use crate::{Command, Response};

use tcp_channel::LittleEndian;
use tcp_channel::{ReceiverBuilder, ChannelRecv};
use tcp_channel::{SenderBuilder, ChannelSend};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::io::{BufReader, BufWriter};
use std::error::Error;
use std::thread::JoinHandle;

fn client_handler_thread(stream: TcpStream, command_sender: Sender<(Sender<Response>, Command)>)
    -> Result<JoinHandle<()>, Box<dyn Error>>
{
    let mut receiver = ReceiverBuilder::new()
        .with_type::<Command>()
        .with_endianness::<LittleEndian>()
        .build(BufReader::new(stream.try_clone()?));

    let mut sender = SenderBuilder::new()
        .with_type::<Response>()
        .with_endianness::<LittleEndian>()
        .build(BufWriter::new(stream.try_clone()?));

    let (response_send, response_recv) = channel::<Response>();

    Ok(std::thread::spawn(move || loop
    {
        match receiver.recv()
        {
            Ok(command) =>
            {
                command_sender.send((response_send.clone(), command)).unwrap();

                let response = response_recv.recv().unwrap();
                sender.send(&response).unwrap();
                if sender.flush().is_err() {
                    break;
                }
            },

            Err(_) => break,
        }
    }))
}

fn server_thread(command_sender: Sender<(Sender<Response>, Command)>, 
                 shutdown_signal: Arc<Mutex<bool>>) 
    -> Result<JoinHandle<()>, Box<dyn Error>>
{
    // FIXME: Allow changing this port
    let listener = TcpListener::bind("0.0.0.0:9988")?;
    
    Ok(std::thread::spawn(move ||
    {
        let mut client_handlers = Vec::<(TcpStream, JoinHandle<()>)>::new();

        loop
        {
            match listener.accept()
            {
                Ok((stream, _socket)) =>
                {
                    if *shutdown_signal.lock().unwrap() {
                        break;
                    }

                    let thread = client_handler_thread(
                        stream.try_clone().unwrap(), command_sender.clone());

                    client_handlers.push((stream, thread.unwrap()));
                },

                Err(err) =>
                {
                    println!("Server Error: {}", err);
                    break;
                },
            }
        }

        // Shutdown clients
        for (stream, thread) in client_handlers 
        {
            stream.shutdown(Shutdown::Both).unwrap();
            thread.join().unwrap();
        }
    }))
}

pub fn start<F>(mut on_command: F) -> Result<(), Box<dyn Error>>
    where F: FnMut(Command) -> Response + Send + 'static
{
    let (command_sender, command_recv) = channel::<(Sender<Response>, Command)>();
    let shutdown_signal = Arc::from(Mutex::from(false));
    let server = server_thread(command_sender, shutdown_signal.clone())?;

    for (response_sender, command) in command_recv 
    {
        let response = on_command(command);
        response_sender.send(response.clone())?;

        if response == Response::Exit {
            break;
        }
    }
    
    // Shutdown server
    *shutdown_signal.lock().unwrap() = true;
    TcpStream::connect("127.0.0.1:9988")?;
    server.join().unwrap();
    
    Ok(())
}
