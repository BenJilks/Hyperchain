/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use super::packet::PacketHandler;
use super::client_manager::ClientManager;

use std::net::TcpListener;
use std::error::Error;
use std::thread::JoinHandle;

pub fn start_server_thread<H>(command_handler: H, mut manager: ClientManager)
    -> Result<JoinHandle<()>, Box<dyn Error>>
    where H: PacketHandler + Clone + Send + Sync + 'static
{
    // FIXME: Allow changing this port
    let listener = TcpListener::bind(format!("0.0.0.0:{}", manager.port()))?;

    info!("[{}] Starting server", manager.port());
    Ok(std::thread::spawn(move || loop
    {
        match listener.accept()
        {
            Ok((stream, socket)) =>
            {
                if manager.should_shutdown() {
                    break;
                }

                let ip = socket.ip().to_string();
                info!("[{}] Got connection from {}", manager.port(), ip);

                manager.new_client(command_handler.clone(), stream, ip).unwrap();
            },

            Err(err) =>
            {
                error!("[{}] Server Error: {}", manager.port(), err);
                break;
            },
        }
    }))
}

