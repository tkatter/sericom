use crate::screen::ScreenBuffer;

pub fn process_erase(seq: &[u8], kind: u8, sb: &mut ScreenBuffer) {
    let body = &seq[2..seq.len() - 1];

    // erase from cursor until end of screen
    if kind == b'J' && (body.is_empty() || body == [b'0']) {
        todo!();
    }

    // erase from cursor until end of line
    if kind == b'K' && (body.is_empty() || body == [b'0']) {
        todo!();
    }

    match (kind, body) {
        // erase from cursor to beginning of screen
        (b'J', [b'1']) => todo!(),
        // erase entire screen
        (b'J', [b'2']) => todo!(),
        // erase saved lines
        (b'J', [b'3']) => todo!(),
        // erase start of line to the cursor
        (b'K', [b'1']) => todo!(),
        // erase the entire line
        (b'K', [b'2']) => todo!(),
        _ => {}
    }

    todo!()
}

pub fn process_screen(seq: &[u8], kind: u8, sb: &mut ScreenBuffer) {
    let body = &seq[2..seq.len() - 1];
    // ESC[={value}h Changes the screen width or type to the mode specified by value

    // erase from cursor until end of screen
    if kind == b'J' && (body.is_empty() || body == [b'0']) {
        todo!();
    }

    // erase from cursor until end of line
    if kind == b'K' && (body.is_empty() || body == [b'0']) {
        todo!();
    }

    match (kind, body) {
        // erase from cursor to beginning of screen
        (b'J', [b'1']) => todo!(),
        // erase entire screen
        (b'J', [b'2']) => todo!(),
        // erase saved lines
        (b'J', [b'3']) => todo!(),
        // erase start of line to the cursor
        (b'K', [b'1']) => todo!(),
        // erase the entire line
        (b'K', [b'2']) => todo!(),
        _ => {}
    }

    todo!()
}
