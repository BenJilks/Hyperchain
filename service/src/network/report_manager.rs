/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use super::packet::PacketHandler;
use super::client_manager::ClientManager;

use std::thread::JoinHandle;
use std::time::Duration;

pub fn start_report_manager_thread<H>(packet_handler: H,
                                      mut manager: ClientManager)
        -> JoinHandle<()>
    where H: PacketHandler + Clone + Send + Sync + 'static
{
    std::thread::spawn(move || loop
    {
        if manager.should_shutdown() 
        {
            debug!("[{}] Exit report manager", manager.port());
            break;
        }

        packet_handler.update_reports(&mut manager);
        std::thread::sleep(Duration::from_millis(1000));
    })
}

