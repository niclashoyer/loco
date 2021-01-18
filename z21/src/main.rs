use bitflags::bitflags;
use embedded_nal::{UdpClient, UdpServer};
use loco_xpressnet as xnet;

bitflags! {
	pub struct CentralStateEx: u8 {
		const HIGH_TEMPERATURE = 0b0000_0001;
		const POWER_LOST = 0b0000_0010;
		const SHORT_CIRCUIT_EXTERNAL = 0b0000_0100;
		const SHORT_CIRCUIT_INTERNAL = 0b0000_1000;
	}
}

bitflags! {
	pub struct CentralState: u8 {
		const EMERGENCY_OFF = 0b0000_0001;
		const EMERGENCY_STOP = 0b0000_0010;
		const SHORT_CIRCUIT = 0b0000_0100;
		const PROGRAMMING_MODE = 0b0010_0000;
	}
}

impl xnet::Bits<u8> for CentralState {
	fn bits(&self) -> u8 {
		self.bits
	}
}

bitflags! {
	pub struct BroadcastFlags: u32 {
		const DRIVING_SWITCHING = 0x00000001;
		const RBUS = 0x00000002;
		const RAILCOM = 0x00000004;
		const SYSTEM_STATUS = 0x00000100;
		const DRIVING_SWITCHING_ALL = 0x00010000;
		const LOCONET = 0x01000000;
		const LOCONET_LOCO = 0x02000000;
		const LOCONET_SWITCH = 0x04000000;
		const LOCONET_OCCUPY = 0x08000000;
		const RAILCOM_ALL = 0x00040000;
		const CAN_OCCUPY = 0x00080000;
	}
}

#[derive(Debug)]
enum HardwareType {
	Z21Old,
	Z21New,
	SmartRail,
	Z21Small,
	Z21Start,
	Custom(u32),
}

#[derive(Debug)]
struct FirmwareVersion {
	major: u8,
	minor: u8,
}

#[derive(Debug)]
enum CentralMessage {
	HardwareInfo(HardwareType, FirmwareVersion),
	SerialNumber(u32),
	SystemState {
		main_current: i16,
		prog_current: i16,
		filtered_main_current: i16,
		temperature: i16,
		supply_voltage: u16,
		vcc_voltage: u16,
		central_state: CentralState,
		central_state_ex: CentralStateEx,
	},
	BroadcastFlags(BroadcastFlags),
	XpressNet(xnet::CentralMessage<CentralState>),
}

impl CentralMessage {
	pub fn to_buf(&self, buf: &mut [u8]) -> usize {
		use CentralMessage::*;
		match self {
			SystemState {
				main_current,
				prog_current,
				filtered_main_current,
				temperature,
				supply_voltage,
				vcc_voltage,
				central_state,
				central_state_ex,
			} => {
				buf[0] = 0x14;
				buf[1] = 0x00;
				buf[2] = 0x84;
				buf[3] = 0x00;
				(&mut buf[4..6]).put_i16_le(*main_current);
				(&mut buf[6..8]).put_i16_le(*prog_current);
				(&mut buf[8..10]).put_i16_le(*filtered_main_current);
				(&mut buf[10..12]).put_i16_le(*temperature);
				(&mut buf[12..14]).put_u16_le(*supply_voltage);
				(&mut buf[14..16]).put_u16_le(*vcc_voltage);
				buf[16] = central_state.bits;
				buf[17] = central_state_ex.bits;
				buf[18] = 0x00;
				buf[19] = 0x00;
				20
			}
			SerialNumber(num) => {
				buf[0] = 0x10;
				buf[1] = 0x11;
				(&mut buf[2..6]).put_u32_le(*num);
				6
			}
			_ => unimplemented!(),
		}
	}
}

trait BufMut {
	fn put_i16_le(&mut self, num: i16);
	fn put_u16_le(&mut self, num: u16);
	fn put_u32_le(&mut self, num: u32);
}

impl BufMut for &mut [u8] {
	fn put_i16_le(&mut self, num: i16) {
		let bytes = num.to_le_bytes();
		self[0] = bytes[0];
		self[1] = bytes[1];
	}

	fn put_u16_le(&mut self, num: u16) {
		let bytes = num.to_le_bytes();
		self[0] = bytes[0];
		self[1] = bytes[1];
	}

	fn put_u32_le(&mut self, num: u32) {
		let bytes = num.to_le_bytes();
		self[0] = bytes[0];
		self[1] = bytes[1];
		self[2] = bytes[2];
		self[3] = bytes[3];
	}
}

#[derive(Debug)]
enum ClientMessage {
	GetHardwareInfo,
	GetSerialNumber,
	GetSystemState,
	GetBroadcastFlags,
	SetBroadcastFlags(BroadcastFlags),
	XpressNet(xnet::DeviceMessage),
}

impl ClientMessage {
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
		use ClientMessage::*;
		println!("{:#04X?}", bytes);
		if bytes.len() < 4 {
			return Err(Error::ParseCommand);
		}
		let len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
		let header = &bytes[2..4];
		match header {
			&[0x40, 0x00] => Ok(XpressNet(xnet::DeviceMessage::from_bytes(&bytes[4..len])?)),
			&[0x85, 0x00] => Ok(GetSystemState),
			&[0x10, 0x00] => Ok(GetSerialNumber),
			_ => Err(Error::ParseCommand),
		}
	}
}

#[derive(Debug)]
enum Error {
	Receive,
	Send,
	Bind,
	ParseCommand,
	XpressNet(xnet::Error),
}

impl From<xnet::Error> for Error {
	fn from(e: xnet::Error) -> Error {
		Error::XpressNet(e)
	}
}

const BUF_SIZE: usize = 64;

struct Server<S>
where
	S: Sized,
{
	socket: S,
	recv_buf: [u8; BUF_SIZE],
	send_buf: [u8; BUF_SIZE],
}

type ClientAddress = embedded_nal::SocketAddr;

impl<S> Server<S>
where
	S: Sized,
{
	pub fn new(socket: S) -> Self {
		Self {
			socket,
			recv_buf: [0; BUF_SIZE],
			send_buf: [0; BUF_SIZE],
		}
	}

	pub fn send<U, E>(
		&mut self,
		server: &U,
		client: ClientAddress,
		message: &CentralMessage,
	) -> nb::Result<(), Error>
	where
		U: UdpServer<Error = E, UdpSocket = S>,
		E: core::fmt::Debug,
	{
		let len = message.to_buf(&mut self.send_buf[2..]);
		(&mut self.send_buf[0..2]).put_u16_le(len as u16);
		server
			.send_to(&mut self.socket, client, &self.send_buf[0..len])
			.map_err(|e| e.map(|_| Error::Send))
	}

	pub fn receive<U, E>(&mut self, server: &U) -> nb::Result<(ClientAddress, ClientMessage), Error>
	where
		U: UdpServer<Error = E, UdpSocket = S>,
		E: core::fmt::Debug,
	{
		let (num, addr) = server
			.receive(&mut self.socket, &mut self.recv_buf)
			.map_err(|e| e.map(|_| Error::Receive))?;
		let msg = ClientMessage::from_bytes(&self.recv_buf[0..num])?;
		Ok((addr, msg))
	}
}

fn main() {
	use nb::block;
	use std_embedded_nal::STACK;

	const PORT: u16 = 21105;

	let mut sock = STACK.socket().unwrap();
	STACK.bind(&mut sock, PORT).unwrap();

	println!("listening on port {}", PORT);

	let mut server = Server::new(sock);
	loop {
		let recv = block!(server.receive(&STACK));
		println!("received: {:?}", recv);
		if let Ok((addr, msg)) = recv {
			use CentralMessage::*;
			use ClientMessage::*;
			match msg {
				GetSystemState => {
					let state = SystemState {
						main_current: 0,
						prog_current: 0,
						filtered_main_current: 0,
						temperature: 20,
						supply_voltage: 18,
						vcc_voltage: 5,
						central_state: CentralState::empty(),
						central_state_ex: CentralStateEx::empty(),
					};
					block!(server.send(&STACK, addr, &state)).unwrap();
				}
				GetSerialNumber => {
					let sn = SerialNumber(42);
					block!(server.send(&STACK, addr, &sn)).unwrap();
				}
				ClientMessage::XpressNet(xnet::DeviceMessage::GetState) => {
					let state = CentralState::empty();
					let msg = CentralMessage::XpressNet(xnet::CentralMessage::State(state));
					//block!(server.send(&STACK, addr, &msg)).unwrap();
				}
				_ => {}
			}
		}
	}
}
