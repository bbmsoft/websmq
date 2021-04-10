use std::{error::Error, fmt::Display, string::FromUtf8Error};

pub type Topic = String;
pub type Payload = Vec<u8>;

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolError(String);

impl Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<FromUtf8Error> for ProtocolError {
    fn from(e: FromUtf8Error) -> Self {
        ProtocolError(format!("{}", e))
    }
}

impl Error for ProtocolError {}

pub type ProtocolResult<T> = std::result::Result<T, ProtocolError>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Message {
    Publish(Topic, Payload),
    Subscribe(Topic),
    Unsubscribe(Topic),
    LastWill(Topic, Payload),
}

pub fn parse_message(buffer: &[u8]) -> ProtocolResult<Message> {
    let topic = get_topic(buffer)?;
    let payload = get_payload(buffer)?;
    let type_byte = &buffer[0];
    let shifted = type_byte >> 6;
    match shifted {
        0b00 => Ok(Message::Publish(topic, payload)),
        0b01 => Ok(Message::Subscribe(topic)),
        0b10 => Ok(Message::Unsubscribe(topic)),
        0b11 => Ok(Message::LastWill(topic, payload)),
        _ => panic!("the compiler does not know, but this pattern IS exhaustive"),
    }
}

pub fn get_topic_length(buffer: &[u8]) -> u16 {
    let header = &buffer[..2];
    let mut length = header[0] as u16;
    length = length & 0b00111111;
    length = length << 8;
    length = length | header[1] as u16;
    length
}

pub fn get_topic(buffer: &[u8]) -> ProtocolResult<String> {
    let topic_length = get_topic_length(buffer);
    let start = 2 as usize;
    let end = start + topic_length as usize;

    if buffer.len() < end {
        return Err(ProtocolError("topic length exceeds buffer size".to_owned()));
    }

    let topic_buffer = &buffer[start..end];
    let topic = String::from_utf8(topic_buffer.to_owned());

    Ok(topic?)
}

pub fn get_payload(buffer: &[u8]) -> ProtocolResult<Vec<u8>> {
    let topic_length = get_topic_length(buffer);
    let topic_start = 2 as usize;
    let payload_start = topic_start + topic_length as usize;

    if buffer.len() < payload_start {
        return Err(ProtocolError("topic length exceeds buffer size".to_owned()));
    }

    let payload_buffer = &buffer[payload_start..];
    Ok(payload_buffer.to_owned())
}

#[cfg(test)]
mod test {
    use super::*;

    const SUBSCRIBE_TO_HELLO: [u8; 7] = [
        0b01000000, 0b00000101, 0b01101000, 0b01100101, 0b01101100, 0b01101100, 0b01101111,
    ];

    const PUBLISH_WORLD_TO_HELLO: [u8; 12] = [
        0b00000000, 0b00000101, 0b01101000, 0b01100101, 0b01101100, 0b01101100, 0b01101111,
        0b01110111, 0b01101111, 0b01110010, 0b01101100, 0b01100100,
    ];

    #[test]
    fn test_get_topic_length() {
        assert_eq!(get_topic_length(&[0b00000000, 0b00000000]), 0);
        assert_eq!(get_topic_length(&[0b01000000, 0b01111011]), 123);
        assert_eq!(get_topic_length(&[0b10111111, 0b11111111]), 16383);
        assert_eq!(get_topic_length(&SUBSCRIBE_TO_HELLO), 5);
    }

    #[test]
    fn test_parse_message() {
        assert_eq!(
            parse_message(&SUBSCRIBE_TO_HELLO).unwrap(),
            Message::Subscribe("hello".to_owned())
        );

        assert_eq!(
            parse_message(&PUBLISH_WORLD_TO_HELLO).unwrap(),
            Message::Publish("hello".to_owned(), "world".as_bytes().to_owned())
        );
    }

    #[test]
    fn test_get_topic() {
        assert_eq!(get_topic(&SUBSCRIBE_TO_HELLO).unwrap(), "hello");
    }

    #[test]
    fn test_get_payload() {
        let payload = get_payload(&SUBSCRIBE_TO_HELLO).unwrap();
        assert_eq!(String::from_utf8(payload).unwrap(), "");
        let payload = get_payload(&PUBLISH_WORLD_TO_HELLO).unwrap();
        assert_eq!(String::from_utf8(payload).unwrap(), "world");
    }
}
