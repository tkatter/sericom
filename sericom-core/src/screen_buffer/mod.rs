mod screen_buffer;
pub use screen_buffer::*;


#[test]
fn test_add_data() -> miette::Result<()> {
    use crate::configs::initialize_config;
    initialize_config()?;

    let mut sb = ScreenBuffer::new(80, 24, 5000);

    let data = b"hello world my name is thomas and this is some test data";
    sb.add_data(data);
    assert!(sb.escape_state == EscapeState::Normal);

    // ASCII ESC character/byte
    sb.add_data(&[0x1B]);
    assert!(sb.escape_state == EscapeState::Esc);

    assert_eq!(b"[", &[0x5B]);
    sb.add_data(&[0x5B]);
    assert!(sb.escape_state == EscapeState::Csi);

    sb.add_data(b"33;32");
    println!("{:?}", sb.escape_sequence);

    assert_eq!(
        EscapeSequence {
            sequence: vec![EscapePart::Numbers(vec![3, 3]), EscapePart::Separator], 
            part: EscapePart::Numbers(vec![3, 2])
        },
        sb.escape_sequence
    );

    sb.add_data(b"A");
    assert!(sb.escape_state == EscapeState::Normal);
    assert_eq!(
        EscapeSequence {
            sequence: vec![], 
            part: EscapePart::Empty
        },
        sb.escape_sequence
    );

    Ok(())
}
