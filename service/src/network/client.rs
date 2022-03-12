/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use super::packet::{Packet, PacketHandler};
use super::packet::{Message, MessageSender, MessageReceiver};
use super::client_manager::ClientManager;

use tcp_channel::LittleEndian;
use tcp_channel::{SenderBuilder, ChannelSend};
use tcp_channel::{ReceiverBuilder, ChannelRecv};
use std::io::{BufReader, BufWriter};
use std::net::TcpStream;
use std::thread::JoinHandle;
use std::error::Error;

fn handle_command<H>(packet: Packet, command_handler: &H, 
                     address: &str, manager: &mut ClientManager)
    where H: PacketHandler
{
    // FIXME: Handler errors
    let _ = command_handler.handle(address, packet, manager);
}

fn request_client_address(
        mut sender: MessageSender, receiver: &mut MessageReceiver,
        ip: &str, manager: &mut ClientManager)
    -> Result<String, Box<dyn Error>>
{
    sender.send(&Message::OnConnected(manager.port()))?;
    sender.flush()?;
    
    match receiver.recv()
    {
        Ok(Message::OnConnected(port)) =>
        {
            let address = format!("{}:{}", ip, port);
            sender.send(&Message::Packet(Packet::OnConnected))?;
            sender.flush()?;

            manager.register_client_sender(address.clone(), sender)?;
            Ok(address)
        }

        _ => panic!(),
    }
}

pub fn client_handler_thread<H>(packet_handler: H, mut manager: ClientManager,
                                stream: TcpStream, ip: String)
    -> Result<JoinHandle<()>, Box<dyn Error>>
    where H: PacketHandler + Send + Sync + 'static
{
    let mut receiver = ReceiverBuilder::new()
        .with_type::<Message>()
        .with_endianness::<LittleEndian>()
        .build(BufReader::new(stream.try_clone()?));
        
    let sender = SenderBuilder::new()
        .with_type::<Message>()
        .with_endianness::<LittleEndian>()
        .build(BufWriter::new(stream.try_clone().unwrap()));

    Ok(std::thread::spawn(move ||
    {
        let address = request_client_address(
            sender, &mut receiver, &ip, &mut manager).unwrap();

        info!("[{}] Connected to {}", manager.port(), address);
        loop
        {
            match receiver.recv()
            {
                // NOTE: We shouldn't be sending an `OnConnected` 
                //       message more then once, do disconnect the 
                //       client, just to be sure.
                Ok(Message::OnConnected(_port)) =>
                    panic!(),

                Ok(Message::KnownNode(node)) => 
                {
                    if manager.register_node(&node) 
                    {
                        manager.send_message_to(Message::KnownNode(node),
                            |x| x != address).unwrap();
                    }
                },

                Ok(Message::Packet(packet)) =>
                {
                    debug!("[{}] Got packet {:?}", manager.port(), packet);
                    handle_command(packet, &packet_handler, 
                        &address, &mut manager);
                },

                // FIXME: Handler errors
                Err(_) => 
                {
                    manager.register_disconnect(&address);
                    break;
                },
            }
        }
    }))
}

