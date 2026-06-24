use cliplink_lib::protocol::{decode, encode, Frame, MessageKind};

#[test]
fn text_frame_round_trips_utf8_payload() {
    let original = Frame::from_text("你好，ClipLink");

    let encoded = encode(&original).expect("text frame encodes");
    let decoded = decode(&encoded).expect("encoded text frame decodes");

    assert_eq!(decoded.kind(), MessageKind::Text);
    assert_eq!(decoded.text().expect("text payload"), "你好，ClipLink");
}
