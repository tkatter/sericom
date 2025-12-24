use super::*;
use crate::assert_event;

#[test]
fn parser_basic() {
    let mut parser = ByteParser::new();
    let bytes = b"\x1b[HI should be home now at 1,1\n\
            This is now the second linr, oops lets change that\x1b[24Gline\x1b[E\
            This should now be the third line. Lets do the next in blue and bold.\n\
            \x1b[1;34mAm I blue now??\x1b[0m\
        ";
    let parsed = parser.feed(bytes);
    assert_event!(parsed[0], CsiKind::PositionCUP, &[]);
    assert_event!(parsed[1], b"I should be home now at 1,1");
    assert_event!(parsed[2], c0 = C0::NL);
    assert_event!(
        parsed[3],
        b"This is now the second linr, oops lets change that"
    );
    assert_event!(parsed[4], CsiKind::CharAbsCHA, b"24");
    assert_event!(parsed[5], b"line");
    assert_event!(parsed[6], CsiKind::NextLine, &[]);
    assert_event!(
        parsed[7],
        b"This should now be the third line. Lets do the next in blue and bold."
    );
    assert_event!(parsed[8], c0 = C0::NL);
    assert_event!(parsed[9], CsiKind::SGR, b"1;34");
    assert_event!(parsed[10], b"Am I blue now??");
    assert_event!(parsed[11], CsiKind::SGR, b"0");
}
