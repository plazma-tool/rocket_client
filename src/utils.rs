use std::mem;

use SyncCmd;

pub fn cmd_to_code(cmd: &SyncCmd) -> u8 {
    use self::SyncCmd::*;
    match *cmd {
        SetKey     => 0,
        DeleteKey  => 1,
        GetTrack   => 2,
        SetRow     => 3,
        Pause      => 4,
        SaveTracks => 5,
        NOOP       => 255,
    }
}

pub fn code_to_cmd(code: u8) -> SyncCmd {
    use self::SyncCmd::*;
    match code {
        0 => SetKey,
        1 => DeleteKey,
        2 => GetTrack,
        3 => SetRow,
        4 => Pause,
        5 => SaveTracks,
        _ => NOOP,
    }
}

/// Convert 4 bytes in network byte order (big-endian) to a little endian u32
pub fn net_to_u32(bytes: &[u8; 4]) -> u32 {
    let mut n: u32 = 0;

    n += (bytes[0] as u32) << 24;
    n += (bytes[1] as u32) << 16;
    n += (bytes[2] as u32) << 8;
    n += (bytes[3] as u32) << 0;

    n
}

/// Convert 4 bytes to f32
pub fn net_to_f32(bytes: &[u8; 4]) -> f32 {
    let number: f32 = unsafe { mem::transmute(net_to_u32(bytes)) };
    number
}

/// Convert an u32 to 4 bytes in network byte order (big-endian)
pub fn u32_to_net(n: u32) -> [u8; 4] {
    let mut bytes = [0; 4];

    bytes[0] = ((n >> 24) & 0xFF) as u8;
    bytes[1] = ((n >> 16) & 0xFF) as u8;
    bytes[2] = ((n >>  8) & 0xFF) as u8;
    bytes[3] = ((n >>  0) & 0xFF) as u8;

    bytes
}

/// Convert an u32 to 4 bytes as little-endian
pub fn u32_to_le(n: u32) -> [u8; 4] {
    let mut bytes = [0; 4];

    bytes[0] = ((n >>  0) & 0xFF) as u8;
    bytes[1] = ((n >>  8) & 0xFF) as u8;
    bytes[2] = ((n >> 16) & 0xFF) as u8;
    bytes[3] = ((n >> 24) & 0xFF) as u8;

    bytes
}

/// Convert an f32 to 4 bytes as little-endian
pub fn f32_to_le(n: f32) -> [u8; 4] {
    let number: u32 = unsafe { mem::transmute(n) };
    u32_to_le(number)
}

/// Returns the time in milliseconds based on the row and rps
pub fn ms_from_row_rps(row: u32, rps: f64) -> u32 {
    let t = ((row as f64) / rps) * 1000.0 + 0.5;
    t as u32
}
