#[macro_use]
extern crate log;
extern crate env_logger;

extern crate rocket_sync;

use std::{fmt, str};
use std::fmt::Display;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::error::Error;

use rocket_sync::{TrackKey, SyncDevice, code_to_key};

pub mod utils;
use utils::*;

const CLIENT_GREET: &'static str = "hello, synctracker!";
const SERVER_GREET: &'static str = "hello, demo!";
const SERVER_GREET_LEN: usize = 12;

pub struct SyncClient {
    stream: TcpStream,
}

pub enum SyncCmd {
    SetKey,
    DeleteKey,
    GetTrack,
    SetRow,
    Pause,
    SaveTracks,
    NOOP,
}

impl SyncClient {

    /// Connects to the Rocket Editor server and shakes hand with it
    pub fn new(address: &str) -> Result<SyncClient, Box<Error>> {
        info!("Attempt to connect to Rocket");
        let mut stream = match TcpStream::connect(address) {
            Ok(x) => {
                info!("Connected to Rocket");
                x
            },
            Err(e) => {
                info!("Couldn't connect to Rocket");
                return Err(Box::new(e));
            },
        };

        // handshake

        // send greeting
        stream.write(CLIENT_GREET.as_bytes())?;

        // receive response
        let mut buf = [0; SERVER_GREET_LEN];
        match stream.read_exact(&mut buf) {
            Ok(_) => {
                match str::from_utf8(&buf) {
                    Ok(x) => {
                        let resp = String::from(x);
                        if String::from(SERVER_GREET) != resp {
                            return Err(Box::new(SyncError::BadServerGreeting));
                        }
                    },
                    Err(_) => {
                        // invalid response, can't parse as utf8
                        return Err(Box::new(SyncError::CantParseGreeting));
                    }
                }
            },

            Err(_) => {
                return Err(Box::new(SyncError::CouldNotReadFromServer));
            }
        }

        info!("Handshake completed");

        Ok(SyncClient{stream: stream})
    }

    /// Read from the stream and process commands until the server runs out of
    /// things to send.
    ///
    /// Returns an Ok(true) if there should be a redraw.
    pub fn update(&mut self, mut device: &mut SyncDevice) -> Result<bool, Box<Error>> {

        let mut draw_anyway = false;

        // Use nonblocking when receiving commands, otherwise it stalls the
        // application until the user does something in the editor again. When
        // the server is out of things to say, reading will error and we will
        // know to stop.
        //
        // Stop nonblocking in the command handlers, take as many bytes as
        // expected for the command, then resume nonblocking until the end of
        // update().

        self.stream.set_nonblocking(true)?;

        let mut is_sending = true;
        while is_sending {
            // this better be an u8 command code
            let mut cmd_buf = [0; 1];

            match self.stream.read(&mut cmd_buf) {
                Ok(n) => {
                    // nothing to read
                    if n == 0 {
                        is_sending = false;
                    }

                    use self::SyncCmd::*;
                    match code_to_cmd(cmd_buf[0]) {
                        NOOP => info!("Received: CMD NOOP, byte {}", cmd_buf[0]),

                        SetKey => self.handle_set_key_cmd(&mut device)?,

                        DeleteKey => self.handle_del_key_cmd(&mut device)?,

                        GetTrack => {
                            info!("Received: CMD GetTrack");
                            info!("TODO handle GetTrack");
                        },

                        SetRow => self.handle_set_row_cmd(&mut device)?,

                        Pause => self.handle_pause_cmd(&mut device)?,

                        SaveTracks => {
                            info!("Received: CMD SaveTracks");
                            info!("TODO handle SaveTracks");
                        },
                    }

                    draw_anyway = match code_to_cmd(cmd_buf[0]) {
                        NOOP => false,
                        SetKey => true,
                        DeleteKey => true,
                        GetTrack => false,
                        SetRow => true,
                        Pause => true,
                        SaveTracks => false,
                    };
                },

                // read error, probably nothing to read
                Err(_) => is_sending = false,
            }
        }

        self.stream.set_nonblocking(false)?;

        Ok(draw_anyway)
    }

    pub fn send_row(&mut self, device: &SyncDevice) -> Result<(), Box<Error>> {
        let buf = [cmd_to_code(&SyncCmd::SetRow)];
        self.stream.write(&buf)?;

        let buf = u32_to_net(device.row);

        info!("Send row: {}, bytes: {:?}", device.row, buf);
        self.stream.write(&buf)?;

        Ok(())
    }

    /// Send track names to Rocket, including group prefix
    pub fn send_track_names(&mut self, track_names: &Vec<String>) -> Result<(), Box<Error>> {
        info!("Sending track names: {:#?}", track_names);

        for name in track_names.iter() {
            info!("Send CMD Get Track: \"{}\"", name);

            // send get track command
            let buf = [cmd_to_code(&SyncCmd::GetTrack)];
            self.stream.write(&buf)?;

            // send track name length
            let buf = u32_to_net(name.len() as u32);
            self.stream.write(&buf)?;

            // send track name
            let buf = name.as_bytes();
            self.stream.write(buf)?;
        }

        Ok(())
    }

    /// Adds a key frame to a track
    pub fn handle_set_key_cmd(&mut self, device: &mut SyncDevice) -> Result<(), Box<Error>> {
        info!("Received: CMD SetKey");

        self.stream.set_nonblocking(false)?;

        let mut track_key: TrackKey = TrackKey::new();

        // track index (to indetify in the Vec<SyncTrack>)
        let mut buf = [0; 4];
        self.stream.read_exact(&mut buf)?;
        let track_idx: usize = net_to_u32(&buf) as usize;

        // row of the key
        let mut buf = [0; 4];
        self.stream.read_exact(&mut buf)?;
        track_key.row = net_to_u32(&buf);

        // value of the key
        let mut buf = [0; 4];
        self.stream.read_exact(&mut buf)?;
        track_key.value = net_to_f32(&buf);

        // interpolation type of the key
        let mut buf = [0; 1];
        self.stream.read_exact(&mut buf)?;
        track_key.key_type = code_to_key(buf[0]);

        self.stream.set_nonblocking(true)?;

        // add the key to the track if it exists

        if device.tracks.len() == 0 {
            return Err(Box::new(SyncError::NoTracks));
        }

        if track_idx > device.tracks.len() {
            return Err(Box::new(SyncError::TrackNotFound));
        }

        device.tracks[track_idx].add_key(track_key);

        Ok(())
    }

    /// Deletes a key from a track
    pub fn handle_del_key_cmd(&mut self, device: &mut SyncDevice) -> Result<(), Box<Error>> {
        info!("Received: CMD DeleteKey");

        self.stream.set_nonblocking(false)?;

        // track index (to indetify in the Vec<SyncTrack>)
        let mut buf = [0; 4];
        self.stream.read_exact(&mut buf)?;
        let track_idx: usize = net_to_u32(&buf) as usize;

        // row of the key
        let mut buf = [0; 4];
        self.stream.read_exact(&mut buf)?;
        let row: u32 = net_to_u32(&buf);

        self.stream.set_nonblocking(true)?;

        if device.tracks.len() == 0 {
            return Err(Box::new(SyncError::NoTracks));
        }

        if track_idx > device.tracks.len() {
            return Err(Box::new(SyncError::TrackNotFound));
        }

        device.tracks[track_idx].delete_key(row);

        Ok(())
    }

    /// Sets the current row from server. Sets the current time based on the row
    /// and rps.
    pub fn handle_set_row_cmd(&mut self, device: &mut SyncDevice) -> Result<(), Box<Error>> {
        info!("Received: CMD SetRow");

        self.stream.set_nonblocking(false)?;

        // get four bytes, sent as big-endian (network byte order)
        let mut buf = [0; 4];
        self.stream.read_exact(&mut buf)?;

        self.stream.set_nonblocking(true)?;

        device.row = net_to_u32(&buf);
        device.time = ms_from_row_rps(device.row, device.rps);
        info!("bytes: {:?}", buf);
        info!("row: {}", device.row);
        info!("time: {}", device.time);

        Ok(())
    }

    pub fn handle_pause_cmd(&mut self, device: &mut SyncDevice) -> Result<(), Box<Error>> {
        info!("Received: CMD Pause");

        self.stream.set_nonblocking(false)?;

        // get one byte, value 1 means paused
        let mut buf = [0; 1];
        self.stream.read_exact(&mut buf)?;

        self.stream.set_nonblocking(true)?;

        device.is_paused = { buf[0] == 1 };
        info!("bytes: {:?}", buf);
        info!("is_paused: {:?}", device.is_paused);

        Ok(())
    }
}

#[derive(Debug)]
pub enum SyncError {
    NotConnected,
    CouldNotConnect,
    BadServerGreeting,
    CantParseGreeting,
    CouldNotReadFromServer,
    TrackNotFound,
    NoTracks,
}

impl Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SyncError: {:?}", self)
    }
}

impl Error for SyncError {
    fn description (&self) -> &str {
        "tell me about it"
    }
}
