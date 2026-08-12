use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u32 = 3;
pub const MAX_MESSAGE_BYTES: usize = 32 * 1024 * 1024;

pub fn typst_version() -> &'static str {
    typst::utils::version().raw()
}

pub fn server_target() -> &'static str {
    env!("WEIBIAN_TARGET")
}

#[derive(Debug, Deserialize)]
pub struct Incoming {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Serialize)]
struct Success<'a> {
    jsonrpc: &'static str,
    id: &'a Value,
    result: Value,
}

#[derive(Debug, Serialize)]
struct Failure<'a> {
    jsonrpc: &'static str,
    id: &'a Value,
    error: RpcError,
}

#[cfg(test)]
#[derive(Debug, Serialize)]
struct Notification<'a> {
    jsonrpc: &'static str,
    method: &'a str,
    params: Value,
}

pub fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<Incoming>> {
    let mut content_length = None;
    let mut saw_header = false;

    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            return if saw_header {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "EOF within JSON-RPC headers",
                ))
            } else {
                Ok(None)
            };
        }
        saw_header = true;
        if line == "\r\n" || line == "\n" {
            break;
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "malformed JSON-RPC header",
            ));
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            let parsed = value.trim().parse::<usize>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid Content-Length")
            })?;
            if parsed > MAX_MESSAGE_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "JSON-RPC message exceeds size limit",
                ));
            }
            content_length = Some(parsed);
        }
    }

    let length = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length header")
    })?;
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    let incoming = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Some(incoming))
}

fn write_value<W: Write, T: Serialize>(writer: &mut W, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())?;
    writer.write_all(&bytes)?;
    writer.flush()
}

pub fn write_success<W: Write>(writer: &mut W, id: &Value, result: Value) -> io::Result<()> {
    write_value(
        writer,
        &Success {
            jsonrpc: "2.0",
            id,
            result,
        },
    )
}

pub fn write_error<W: Write>(writer: &mut W, id: &Value, error: RpcError) -> io::Result<()> {
    write_value(
        writer,
        &Failure {
            jsonrpc: "2.0",
            id,
            error,
        },
    )
}

#[cfg(test)]
fn write_notification<W: Write>(writer: &mut W, method: &str, params: Value) -> io::Result<()> {
    write_value(
        writer,
        &Notification {
            jsonrpc: "2.0",
            method,
            params,
        },
    )
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use serde_json::json;

    use super::*;

    fn framed(value: Value) -> Vec<u8> {
        let body = serde_json::to_vec(&value).unwrap();
        let mut message =
            format!("Content-Length: {}\r\nX-Test: yes\r\n\r\n", body.len()).into_bytes();
        message.extend(body);
        message
    }

    #[test]
    fn reads_unicode_and_concatenated_messages() {
        let one = json!({"jsonrpc":"2.0","id":1,"method":"echo","params":{"text":"世界🙂"}});
        let two = json!({"jsonrpc":"2.0","method":"exit"});
        let mut bytes = framed(one);
        bytes.extend(framed(two));
        let mut reader = BufReader::new(Cursor::new(bytes));

        let first = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(first.params["text"], "世界🙂");
        assert_eq!(
            read_message(&mut reader).unwrap().unwrap().method.unwrap(),
            "exit"
        );
        assert!(read_message(&mut reader).unwrap().is_none());
    }

    #[test]
    fn rejects_missing_length_and_oversized_messages() {
        let mut missing = BufReader::new(Cursor::new(b"X: y\r\n\r\n{}"));
        assert_eq!(
            read_message(&mut missing).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let bytes = format!("Content-Length: {}\r\n\r\n", MAX_MESSAGE_BYTES + 1);
        let mut oversized = BufReader::new(Cursor::new(bytes));
        assert_eq!(
            read_message(&mut oversized).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn writes_byte_accurate_content_length() {
        let mut output = vec![];
        write_notification(&mut output, "note", json!({"text":"🙂"})).unwrap();
        let mut reader = BufReader::new(Cursor::new(output));
        let message = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(message.params["text"], "🙂");
    }
}
