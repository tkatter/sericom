use crossterm::style::{Attribute, Attributes, Color};

use crate::ui::{BK, ColorState, ESC, SEP};

pub fn is_graphics_seq(seq: &[u8]) -> bool {
    seq.len() >= 3
        && seq[0] == ESC
        && seq[1] == BK
        && *seq.last().expect("Verified len != 0") == b'm'
}

pub fn process_colors(seq: &[u8], color_state: &mut ColorState, attrs: &mut Attributes) {
    // Get the part between 'ESC[' and 'm'
    let body = &seq[2..seq.len() - 1];
    let mut body_idx = 0;
    let mut parts_iter = body.split(|&p| p == SEP).peekable();

    while let Some(part) = parts_iter.next() {
        match part {
            // Give any 38;5;26 / 48;2;50;60;70 sequences to crossterm
            [b'3' | b'4', b'8'] => {
                if let Some(ident) = parts_iter.next_if(|n| *n == [b'5']) {
                    let Some(color) = parts_iter.next() else {
                        break;
                    };

                    // Get slice from body (+ 1 for the separators ';')
                    let part_str_len = part.len() + 1 + ident.len() + 1 + color.len();
                    let slice = &body[body_idx..body_idx + part_str_len];

                    #[cfg(test)]
                    eprintln!("slice: {}", str::from_utf8(slice).unwrap());

                    if let Ok(s) = std::str::from_utf8(slice) {
                        color_state.set_colors(s);
                    }

                    // add to body_idx the indexes we consumed
                    body_idx += part_str_len - part.len();
                } else if let Some(ident) = parts_iter.next_if(|n| *n == [b'2']) {
                    let Some(r) = parts_iter.next() else { break };
                    let Some(g) = parts_iter.next() else { break };
                    let Some(b) = parts_iter.next() else { break };

                    // get slice from body (+ 1 for the separators ';')
                    let part_str_len =
                        part.len() + 1 + ident.len() + 1 + r.len() + 1 + g.len() + 1 + b.len();
                    let slice = &body[body_idx..body_idx + part_str_len];

                    #[cfg(test)]
                    eprintln!("slice: {}", str::from_utf8(slice).unwrap());

                    if let Ok(s) = std::str::from_utf8(slice) {
                        color_state.set_colors(s);
                    }

                    // add to body_idx the indexes we consumed
                    body_idx += part_str_len - part.len();
                }
            }
            _ => handle_colors_and_attrs(part, color_state, attrs),
        }
        // + 1 for the separator
        body_idx += part.len() + 1;
    }
}

fn handle_colors_and_attrs(body: &[u8], color_state: &mut ColorState, attrs: &mut Attributes) {
    match *body {
        // Attribute sgr mapping to crossterm
        [b'0'] => {
            *color_state = ColorState::default(); // uses config colors
            *attrs = Attributes::default();
        }
        [b'1'] => attrs.set(Attribute::Bold),
        [b'2'] => attrs.set(Attribute::Dim),
        [b'3'] => attrs.set(Attribute::Italic),
        [b'4'] => attrs.set(Attribute::Underlined),
        [b'4', b':', b'2'] => attrs.set(Attribute::DoubleUnderlined),
        [b'4', b':', b'3'] => attrs.set(Attribute::Undercurled),
        [b'4', b':', b'4'] => attrs.set(Attribute::Underdotted),
        [b'4', b':', b'5'] => attrs.set(Attribute::Underdashed),
        [b'5'] => attrs.set(Attribute::SlowBlink),
        [b'6'] => attrs.set(Attribute::RapidBlink),
        [b'7'] => attrs.set(Attribute::Reverse),
        [b'8'] => attrs.set(Attribute::Hidden),
        [b'9'] => attrs.set(Attribute::CrossedOut),
        [b'2', b'0'] => attrs.set(Attribute::Fraktur),
        [b'2', b'1'] => attrs.set(Attribute::NoBold),
        [b'2', b'2'] => attrs.set(Attribute::NormalIntensity),
        [b'2', b'3'] => attrs.set(Attribute::NoItalic),
        [b'2', b'4'] => attrs.set(Attribute::NoUnderline),
        [b'2', b'5'] => attrs.set(Attribute::NoBlink),
        [b'2', b'7'] => attrs.set(Attribute::NoReverse),
        [b'2', b'8'] => attrs.set(Attribute::NoHidden),
        [b'2', b'9'] => attrs.set(Attribute::NotCrossedOut),
        [b'5', b'1'] => attrs.set(Attribute::Framed),
        [b'5', b'2'] => attrs.set(Attribute::Encircled),
        [b'5', b'3'] => attrs.set(Attribute::OverLined),
        [b'5', b'4'] => attrs.set(Attribute::NotFramedOrEncircled),
        [b'5', b'5'] => attrs.set(Attribute::NotOverLined),
        // Basic 8 foreground
        [b'3', b'0'] => color_state.set_fg(Color::Black),
        [b'3', b'1'] => color_state.set_fg(Color::DarkRed),
        [b'3', b'2'] => color_state.set_fg(Color::DarkGreen),
        [b'3', b'3'] => color_state.set_fg(Color::DarkYellow),
        [b'3', b'4'] => color_state.set_fg(Color::DarkBlue),
        [b'3', b'5'] => color_state.set_fg(Color::DarkMagenta),
        [b'3', b'6'] => color_state.set_fg(Color::DarkCyan),
        [b'3', b'7'] => color_state.set_fg(Color::Grey),
        // Basic 8 background
        [b'4', b'0'] => color_state.set_bg(Color::Black),
        [b'4', b'1'] => color_state.set_bg(Color::DarkRed),
        [b'4', b'2'] => color_state.set_bg(Color::DarkGreen),
        [b'4', b'3'] => color_state.set_bg(Color::DarkYellow),
        [b'4', b'4'] => color_state.set_bg(Color::DarkBlue),
        [b'4', b'5'] => color_state.set_bg(Color::DarkMagenta),
        [b'4', b'6'] => color_state.set_bg(Color::DarkCyan),
        [b'4', b'7'] => color_state.set_bg(Color::Grey),
        // Basic 8 bright foreground
        [b'9', b'0'] => color_state.set_fg(Color::DarkGrey),
        [b'9', b'1'] => color_state.set_fg(Color::Red),
        [b'9', b'2'] => color_state.set_fg(Color::Green),
        [b'9', b'3'] => color_state.set_fg(Color::Yellow),
        [b'9', b'4'] => color_state.set_fg(Color::Blue),
        [b'9', b'5'] => color_state.set_fg(Color::Magenta),
        [b'9', b'6'] => color_state.set_fg(Color::Cyan),
        [b'9', b'7'] => color_state.set_fg(Color::White),
        // Basic 8 bright background
        [b'1', b'0', b'0'] => color_state.set_bg(Color::DarkGrey),
        [b'1', b'0', b'1'] => color_state.set_bg(Color::Red),
        [b'1', b'0', b'2'] => color_state.set_bg(Color::Green),
        [b'1', b'0', b'3'] => color_state.set_bg(Color::Yellow),
        [b'1', b'0', b'4'] => color_state.set_bg(Color::Blue),
        [b'1', b'0', b'5'] => color_state.set_bg(Color::Magenta),
        [b'1', b'0', b'6'] => color_state.set_bg(Color::Cyan),
        [b'1', b'0', b'7'] => color_state.set_bg(Color::White),
        // Set colors to default
        [b'3', b'9'] => color_state.set_fg(Color::Reset),
        [b'4', b'9'] => color_state.set_bg(Color::Reset),
        _ => {} // Ignore rest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        configs::{ConfigOverride, get_config, initialize_config},
        ui::ColorState,
    };
    use crossterm::style::{Attribute, Attributes, Color};

    const CONF_OR: ConfigOverride = ConfigOverride {
        color: None,
        out_dir: None,
        exit_script: None,
    };

    struct Case<'a> {
        seq: &'a [u8],
        expected_fg: Option<Color>,
        expected_bg: Option<Color>,
        expected_attrs: Vec<Attribute>,
        label: &'a str,
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn has_all(attrs: &Attributes, expected: &[Attribute]) -> bool {
        expected.iter().all(|a| attrs.has(*a))
    }

    fn test_cases(cases: &Vec<Case>) {
        for (idx, case) in cases.iter().enumerate() {
            let mut color_state = ColorState::default();
            let mut attrs = Attributes::default();

            process_colors(case.seq, &mut color_state, &mut attrs);

            assert_eq!(
                color_state.get_colors().foreground,
                case.expected_fg,
                "Case# {} - Foreground mismatch on '{}'",
                idx,
                case.label
            );
            assert_eq!(
                color_state.get_colors().background,
                case.expected_bg,
                "Case# {} - Background mismatch on '{}'",
                idx,
                case.label
            );
            assert!(
                has_all(&attrs, &case.expected_attrs),
                "Case# {} - Missing attrs on '{}': expected {:?}, got {:?}",
                idx,
                case.label,
                case.expected_attrs,
                attrs
            );
        }
    }

    #[test]
    fn test_basic_fg() {
        initialize_config(CONF_OR).ok();
        let config = get_config();
        let bg = Color::from(&config.appearance.bg);

        let cases = vec![
            Case {
                seq: b"\x1b[31m",
                expected_fg: Some(Color::DarkRed),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "FG basic red",
            },
            Case {
                seq: b"\x1b[37m",
                expected_fg: Some(Color::Grey),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "FG basic white/grey",
            },
        ];
        test_cases(&cases);
    }

    #[test]
    fn test_basic_bg() {
        initialize_config(CONF_OR).ok();
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);

        let cases = vec![Case {
            seq: b"\x1b[44m",
            expected_fg: Some(fg),
            expected_bg: Some(Color::DarkBlue),
            expected_attrs: vec![],
            label: "BG basic blue",
        }];
        test_cases(&cases);
    }

    #[test]
    fn test_bright_colors() {
        initialize_config(CONF_OR).ok();
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);

        let cases = vec![
            Case {
                seq: b"\x1b[95m",
                expected_fg: Some(Color::Magenta),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "FG bright magenta",
            },
            Case {
                seq: b"\x1b[106m",
                expected_fg: Some(fg),
                expected_bg: Some(Color::Cyan),
                expected_attrs: vec![],
                label: "BG bright cyan",
            },
        ];
        test_cases(&cases);
    }

    #[test]
    fn test_resets_and_defaults() {
        initialize_config(CONF_OR).ok();
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);

        let cases = vec![
            Case {
                seq: b"\x1b[0m",
                expected_fg: Some(fg),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "Reset",
            },
            Case {
                seq: b"\x1b[39m",
                expected_fg: Some(Color::Reset),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "Reset FG default",
            },
            Case {
                seq: b"\x1b[49m",
                expected_fg: Some(fg),
                expected_bg: Some(Color::Reset),
                expected_attrs: vec![],
                label: "Reset BG default",
            },
        ];
        test_cases(&cases);
    }

    #[test]
    fn test_attributes() {
        initialize_config(CONF_OR).ok();
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);

        let cases = vec![Case {
            seq: b"\x1b[1;3;4m",
            expected_fg: Some(fg),
            expected_bg: Some(bg),
            expected_attrs: vec![Attribute::Bold, Attribute::Italic, Attribute::Underlined],
            label: "Bold + Italic + Underlined",
        }];
        test_cases(&cases);
    }

    #[test]
    fn test_256_color_palette() {
        initialize_config(CONF_OR).ok();
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);

        let cases = vec![
            Case {
                seq: b"\x1b[38;5;196m",
                expected_fg: Some(Color::AnsiValue(196)),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "FG 256 red",
            },
            Case {
                seq: b"\x1b[48;5;27m",
                expected_fg: Some(fg),
                expected_bg: Some(Color::AnsiValue(27)),
                expected_attrs: vec![],
                label: "BG 256 blue",
            },
        ];
        test_cases(&cases);
    }

    #[test]
    fn test_truecolor_palette() {
        initialize_config(CONF_OR).ok();
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);

        let cases = vec![
            Case {
                seq: b"\x1b[38;2;255;128;64m",
                expected_fg: Some(Color::Rgb {
                    r: 255,
                    g: 128,
                    b: 64,
                }),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "FG truecolor orange",
            },
            Case {
                seq: b"\x1b[48;2;10;20;30m",
                expected_fg: Some(fg),
                expected_bg: Some(Color::Rgb {
                    r: 10,
                    g: 20,
                    b: 30,
                }),
                expected_attrs: vec![],
                label: "BG truecolor dark",
            },
        ];
        test_cases(&cases);
    }

    #[test]
    fn test_mix_attr_colors() {
        initialize_config(CONF_OR).ok();
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);

        let cases = vec![
            Case {
                seq: b"\x1b[1;3;4;38;5;202m",
                expected_fg: Some(Color::AnsiValue(202)),
                expected_bg: Some(bg),
                expected_attrs: vec![Attribute::Bold, Attribute::Italic, Attribute::Underlined],
                label: "Bold + Italic + Underlined + FG 256 orange",
            },
            Case {
                seq: b"\x1b[5;7;48;2;128;64;200m",
                expected_fg: Some(fg),
                expected_bg: Some(Color::Rgb {
                    r: 128,
                    g: 64,
                    b: 200,
                }),
                expected_attrs: vec![Attribute::SlowBlink, Attribute::Reverse],
                label: "Blink + Reverse + BG truecolor purple",
            },
        ];
        test_cases(&cases);
    }

    #[test]
    fn test_kitchen_sink() {
        initialize_config(CONF_OR).ok();
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);

        let cases = vec![
            Case {
                seq: b"\x1b[1;3;38;5;202;4;48;2;10;20;30m",
                expected_fg: Some(Color::AnsiValue(202)),
                expected_bg: Some(Color::Rgb {
                    r: 10,
                    g: 20,
                    b: 30,
                }),
                expected_attrs: vec![Attribute::Bold, Attribute::Italic, Attribute::Underlined],
                label: "Bold + Italic + Underlined + FG 256 + BG truecolor",
            },
            Case {
                seq: b"\x1b[20;53m",
                expected_fg: Some(fg),
                expected_bg: Some(bg),
                expected_attrs: vec![Attribute::Fraktur, Attribute::OverLined],
                label: "Fraktur + Overlined",
            },
        ];
        test_cases(&cases);
    }

    #[test]
    fn test_invalid() {
        initialize_config(CONF_OR).ok();
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);

        let cases = vec![
            Case {
                seq: b"\x1b[38;5m",
                expected_fg: Some(fg),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "Incomplete 256 FG (missing index)",
            },
            Case {
                seq: b"\x1b[48;5;999m",
                expected_fg: Some(fg),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "Out-of-range 256 BG (999)",
            },
            Case {
                seq: b"\x1b[38;2;255;0m",
                expected_fg: Some(fg),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "Incomplete truecolor FG (missing B)",
            },
            Case {
                seq: b"\x1b[48;2;256;256;256m",
                expected_fg: Some(fg),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "Invalid RGB components (>255)",
            },
            Case {
                seq: b"\x1b[999m",
                expected_fg: Some(fg),
                expected_bg: Some(bg),
                expected_attrs: vec![],
                label: "Unknown SGR param",
            },
        ];
        test_cases(&cases);
    }
}
