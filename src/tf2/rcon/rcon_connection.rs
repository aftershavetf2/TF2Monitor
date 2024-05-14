#![allow(dead_code)]
use std::io::prelude::*;
use std::net::TcpStream;
use std::time::Duration;

use crate::utils::BoxResult;

#[derive(Debug, Clone)]
pub struct RConArgs {
    pub ip: String,
    pub port: u16,
    pub password: String,
}

impl RConArgs {
    pub fn new() -> Self {
        RConArgs {
            ip: "127.0.0.1".to_string(),
            port: 40434,
            password: "rconpwd".to_string(),
        }
    }
}

pub struct RConConnection {
    args: RConArgs,
    stream: TcpStream,
    id: u32,
}

const PKT_TYPE_RESPONSE_VALUE: u32 = 0;
const PKT_TYPE_EXEC_COMMAND: u32 = 2;
const PKT_TYPE_AUTH_RESPONSE: u32 = 2;
const PKT_TYPE_AUTH: u32 = 3;

// 4 + 4 + 2 = 10. The size of the two u32s and 2 nul bytes.
const PKT_HEADER_SIZE: usize = 4 + 4 + 2;

#[derive(Debug)]
struct Pkt {
    id: u32,
    typ: u32,
    body: String,
}

/// RConConnection implements an synchronous utf8 Source RCon.
/// It supports multi-package responses.
/// Here's how to use this:
///
///    let rcon_args = RConArgs::new();
///    let mut client = rcon::RConConnection::new(&rcon_args).unwrap();
///    client.authorize().unwrap();
///    println!("{}", client.exec_command(&"cvarlist".to_string()).unwrap());
///
impl RConConnection {
    pub fn new(args: &RConArgs) -> BoxResult<Self> {
        let addr = format!("{}:{}", args.ip, args.port);
        let stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(1)))?;
        stream.set_write_timeout(Some(Duration::from_secs(1)))?;

        Ok(RConConnection {
            args: args.clone(),
            stream,
            id: 0,
        })
    }

    pub fn authorize(&mut self) -> BoxResult<()> {
        let pkt = Pkt {
            id: self.next_id(),
            typ: PKT_TYPE_AUTH,
            body: self.args.password.clone(),
        };

        self.write_pkt(&pkt)?;
        loop {
            let reply = self.read_pkt()?;
            if reply.typ == PKT_TYPE_AUTH_RESPONSE {
                // TODO: Check for bad auth
                break;
            }
        }

        Ok(())
    }

    pub fn exec_command(&mut self, cmd: &str) -> BoxResult<String> {
        let request_pkt = Pkt {
            id: self.next_id(),
            typ: PKT_TYPE_EXEC_COMMAND,
            body: cmd.to_string(),
        };

        self.write_pkt(&request_pkt)?;

        // Send a halt package so multi-package replies
        // have a way to know when to stop looping.
        let halt_id = self.send_halt_pkt()?;

        let mut result = String::new();
        loop {
            // Read and accumulate response bodies
            // until a package arrives with the ID
            // of the halt package.
            let reply = self.read_pkt()?;
            if reply.id == halt_id {
                break;
            }
            result.push_str(reply.body.as_str());
        }

        Ok(result)
    }

    /// Increases the package id but limits it to 0-65535,
    /// and then returns the id.
    fn next_id(&mut self) -> u32 {
        self.id = self.id.wrapping_add(1) & 0xffff;
        self.id
    }

    /// Sends a halt/marker package.
    /// A halt package is an empty EXEC_COMMAND whose
    /// only purpose is to allow us to read reply packages
    /// until a reply package has the ID of the sent halt package.
    fn send_halt_pkt(&mut self) -> BoxResult<u32> {
        let halt_pkt = Pkt {
            id: self.id.wrapping_add(1) & 0xffff,
            typ: PKT_TYPE_EXEC_COMMAND,
            body: "".to_string(),
        };

        self.write_pkt(&halt_pkt)?;

        Ok(halt_pkt.id)
    }

    fn read_pkt(&mut self) -> BoxResult<Pkt> {
        let size: u32 = self.read_u32()?;
        let id: u32 = self.read_u32()?;
        let typ: u32 = self.read_u32()?;

        let body_len = size as usize - PKT_HEADER_SIZE;
        let body = self.read_string(body_len)?;

        // Nul-termination for the body
        let _nul1 = self.read_u8()?;

        // Extra nul because of spec
        let _nul2 = self.read_u8()?;

        Ok(Pkt { id, typ, body })
    }

    fn write_pkt(&mut self, pkt: &Pkt) -> BoxResult<()> {
        let size = PKT_HEADER_SIZE as u32 + pkt.body.len() as u32;
        let typ = pkt.typ;

        self.write_u32(size)?;
        self.write_u32(pkt.id)?;
        self.write_u32(typ)?;
        self.write_string(&pkt.body)?;
        self.write_u8(0)?;
        self.write_u8(0)?;

        Ok(())
    }

    //
    // Helper function to read/write u32, u8 and String
    //

    fn write_u32(&mut self, value: u32) -> BoxResult<()> {
        self.stream.write_all(value.to_le_bytes().as_ref())?;

        Ok(())
    }

    fn write_u8(&mut self, value: u8) -> BoxResult<()> {
        self.stream.write_all(value.to_le_bytes().as_ref())?;

        Ok(())
    }

    fn write_string(&mut self, value: &str) -> BoxResult<()> {
        self.stream.write_all(value.as_ref())?;

        Ok(())
    }

    fn read_u32(&mut self) -> BoxResult<u32> {
        let mut buf = [0u8; 4];
        self.stream.read_exact(&mut buf)?;

        Ok(u32::from_le_bytes(buf))
    }

    fn read_u8(&mut self) -> BoxResult<u8> {
        let mut buf = [0u8; 1];
        self.stream.read_exact(&mut buf)?;

        Ok(buf[0])
    }

    fn read_string(&mut self, len: usize) -> BoxResult<String> {
        let mut buf: Vec<u8> = vec![0; len];
        self.stream.read_exact(&mut buf)?;

        Ok(String::from_utf8(buf)?)
    }
}
