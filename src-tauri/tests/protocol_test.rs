use cliplink_lib::protocol::{decode, encode, try_decode, Frame, MessageKind};

#[test]
fn text_frame_round_trips_utf8_payload() {
    let original = Frame::from_text("你好，ClipLink");

    let encoded = encode(&original).expect("text frame encodes");
    let decoded = decode(&encoded).expect("encoded text frame decodes");

    assert_eq!(decoded.kind(), MessageKind::Text);
    assert_eq!(decoded.text().expect("text payload"), "你好，ClipLink");
}

#[test]
fn stream_decoder_waits_for_the_complete_frame() {
    let encoded = encode(&Frame::from_text("fragmented")).expect("frame encodes");
    let split_at = encoded.len() - 2;
    let mut buffer = encoded[..split_at].to_vec();

    assert!(try_decode(&mut buffer).expect("partial input is valid").is_none());
    assert_eq!(buffer.len(), split_at);

    buffer.extend_from_slice(&encoded[split_at..]);
    let frame = try_decode(&mut buffer)
        .expect("complete input is valid")
        .expect("complete frame is available");
    assert_eq!(frame.text().expect("text payload"), "fragmented");
    assert!(buffer.is_empty());
}
