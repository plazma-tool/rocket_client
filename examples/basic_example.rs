use std::str;
use std::thread::sleep;
use std::time::Duration;
use std::process::exit;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate rocket_sync;
extern crate rocket_client;

use rocket_sync::{SyncDevice, SyncTrack};
use rocket_client::SyncClient;

fn main() {
    env_logger::init().unwrap();
    info!("main: started");

    // Connect to the Rocket Editor (server process)
    let mut rocket: SyncClient = match SyncClient::new("localhost:1338") {
        Ok(x) => { info!("Connected to Rocket"); x },
        Err(_) => { println!("Couldn't connect to Rocket"); exit(2); },
    };

    let track_names = vec![
        "group0#track0".to_owned(),
        "group0#track1".to_owned(),
        "group0#track2".to_owned(),
        "group1#track0".to_owned(),
        "group1#track1".to_owned(),
        "group1#track2".to_owned(),
    ];

    rocket.send_track_names(&track_names).unwrap();

    let mut sync_device: SyncDevice = SyncDevice::new(125.0, 8);
    sync_device.is_paused = true;

    // add empty tracks
    for _ in track_names.iter() {
        sync_device.tracks.push(SyncTrack::new());
    }

    loop {
        // talk to Rocket
        match rocket.update(&mut sync_device) {
            Ok(_) => {},
            Err(err) => {
                // It's a Box<Error>, so we can't restore the original type.
                // Let's parse the debug string for now.
                let msg: &str = &format!("{:?}", err);
                if msg.contains("kind: UnexpectedEof") {
                    println!("Rocket disconnected. Exiting.");
                    exit(2);
                } else {
                    error!("{}", msg);
                }
            },
        }

        if !sync_device.is_paused {
            match rocket.send_row(&mut sync_device) {
                Ok(_) => {},
                Err(e) => warn!("{:?}", e),
            }
        }

        // calculate track values and print
        println!("Row: {}, Time: {}", sync_device.row, sync_device.time);

        for (idx, track) in sync_device.tracks.iter().enumerate() {
            println!("Track {} : {:>10.5}", track_names[idx], track.value_at(sync_device.row));
        }

        // update time
        if !sync_device.is_paused {
            sync_device.time += 16;// 1s / 60 frames
            sync_device.set_row_from_time();
        }

        // sleep a bit
        sleep(Duration::from_millis(16));

    }

}


