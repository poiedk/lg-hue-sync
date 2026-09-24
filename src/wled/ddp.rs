use anyhow::{bail, Context, Result};
use std::net::{IpAddr, SocketAddr, UdpSocket};

use crate::color::RgbColor;

const DDP_VERSION_1: u8 = 0x40;
const DDP_PUSH: u8 = 0x01;
const DDP_DATA_TYPE_RGB24: u8 = 0x0B;
const DDP_HEADER_LEN: usize = 10;

// WLED's DDP receiver uses 1440 channels per packet: 480 RGB LEDs.
const DDP_MAX_DATA_LEN: usize = 1440;

pub struct WledDdpStreamer {
    socket: UdpSocket,
    target_addr: SocketAddr,
    destination_id: u8,
    sequence: u8,
}

impl WledDdpStreamer {
    pub fn new(target_ip: &str, target_port: u16, destination_id: u8) -> Result<Self> {
        if destination_id == 0 {
            bail!("DDP destination ID 0 is invalid; use 1 for the default WLED display");
        }

        let socket =
            UdpSocket::bind("0.0.0.0:0").context("Failed to bind UDP socket for WLED DDP")?;
        socket
            .set_nonblocking(true)
            .context("Failed to configure WLED DDP socket as nonblocking")?;

        let target_ip: IpAddr = target_ip
            .parse()
            .with_context(|| format!("Invalid WLED DDP target IP: {}", target_ip))?;
        let target_addr = SocketAddr::new(target_ip, target_port);
        socket
            .connect(target_addr)
            .with_context(|| format!("Failed to connect WLED DDP socket to {}", target_addr))?;

        Ok(Self {
            socket,
            target_addr,
            destination_id,
            sequence: 0,
        })
    }

    pub fn send_frame(&mut self, colors: &[RgbColor]) -> Result<()> {
        let (packets, final_sequence) =
            encode_ddp_frame(colors, self.sequence, self.destination_id);
        self.sequence = final_sequence;

        for packet in packets {
            match self.socket.send(&packet) {
                Ok(_) => {}
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.kind() == std::io::ErrorKind::Interrupted =>
                {
                    // Realtime output is lossy by design. The next full frame repairs state.
                    return Ok(());
                }
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("Failed to send WLED DDP frame to {}", self.target_addr)
                    });
                }
            }
        }

        Ok(())
    }
}

fn next_sequence(previous: u8) -> u8 {
    // Keep zero for "sequence not used" out of the live cycle and wrap 1..15.
    if previous >= 15 {
        1
    } else {
        previous + 1
    }
}

fn encode_ddp_frame(
    colors: &[RgbColor],
    previous_sequence: u8,
    destination_id: u8,
) -> (Vec<Vec<u8>>, u8) {
    let mut rgb = Vec::with_capacity(colors.len() * 3);
    for color in colors {
        rgb.extend_from_slice(&[color.r, color.g, color.b]);
    }

    let packet_count = if rgb.is_empty() {
        1
    } else {
        (rgb.len() + DDP_MAX_DATA_LEN - 1) / DDP_MAX_DATA_LEN
    };
    let mut packets = Vec::with_capacity(packet_count);
    let mut offset = 0usize;
    let mut sequence = previous_sequence;

    if rgb.is_empty() {
        sequence = next_sequence(sequence);
        packets.push(encode_packet(&[], sequence, destination_id, 0, true));
        return (packets, sequence);
    }

    while offset < rgb.len() {
        let end = (offset + DDP_MAX_DATA_LEN).min(rgb.len());
        let is_last = end == rgb.len();
        sequence = next_sequence(sequence);
        packets.push(encode_packet(
            &rgb[offset..end],
            sequence,
            destination_id,
            offset as u32,
            is_last,
        ));
        offset = end;
    }

    (packets, sequence)
}

fn encode_packet(
    data: &[u8],
    sequence: u8,
    destination_id: u8,
    channel_offset: u32,
    push: bool,
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(DDP_HEADER_LEN + data.len());
    let flags = DDP_VERSION_1 | if push { DDP_PUSH } else { 0 };

    packet.push(flags);
    packet.push(sequence & 0x0F);
    packet.push(DDP_DATA_TYPE_RGB24);
    packet.push(destination_id);
    packet.extend_from_slice(&channel_offset.to_be_bytes());
    packet.extend_from_slice(&(data.len() as u16).to_be_bytes());
    packet.extend_from_slice(data);
    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_single_packet_rgb24_frame() {
        let colors = [RgbColor::new(255, 128, 64), RgbColor::new(10, 20, 30)];
        let (packets, final_sequence) = encode_ddp_frame(&colors, 6, 1);

        assert_eq!(packets.len(), 1);
        assert_eq!(final_sequence, 7);
        let packet = &packets[0];
        assert_eq!(packet[0], DDP_VERSION_1 | DDP_PUSH);
        assert_eq!(packet[1], 7);
        assert_eq!(packet[2], DDP_DATA_TYPE_RGB24);
        assert_eq!(packet[3], 1);
        assert_eq!(&packet[4..8], &[0, 0, 0, 0]);
        assert_eq!(&packet[8..10], &[0, 6]);
        assert_eq!(&packet[10..], &[255, 128, 64, 10, 20, 30]);
    }

    #[test]
    fn splits_frames_above_480_leds_and_pushes_only_last_packet() {
        let colors = vec![RgbColor::new(1, 2, 3); 481];
        let (packets, final_sequence) = encode_ddp_frame(&colors, 14, 1);

        assert_eq!(packets.len(), 2);
        assert_eq!(final_sequence, 1);
        assert_eq!(packets[0][0], DDP_VERSION_1);
        assert_eq!(packets[0][1], 15);
        assert_eq!(&packets[0][4..8], &[0, 0, 0, 0]);
        assert_eq!(&packets[0][8..10], &[0x05, 0xA0]);

        assert_eq!(packets[1][0], DDP_VERSION_1 | DDP_PUSH);
        assert_eq!(packets[1][1], 1);
        assert_eq!(&packets[1][4..8], &[0, 0, 5, 0xA0]);
        assert_eq!(&packets[1][8..10], &[0, 3]);
        assert_eq!(&packets[1][10..], &[1, 2, 3]);
    }

    #[test]
    fn sequence_wraps_without_using_zero() {
        assert_eq!(next_sequence(0), 1);
        assert_eq!(next_sequence(14), 15);
        assert_eq!(next_sequence(15), 1);
    }
}
