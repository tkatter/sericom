use crate::configs::get_config;
use crate::screen::process::*;
use crossterm::style::{Attribute, Attributes, Color};

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
    crate::configs::init_for_tests();
    let config = get_config().unwrap();
    let bg = Color::from(&config.appearance.bg);
    drop(config);

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
    crate::configs::init_for_tests();
    let config = get_config().unwrap();
    let fg = Color::from(&config.appearance.fg);
    drop(config);

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
    crate::configs::init_for_tests();
    let config = get_config().unwrap();
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

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
    crate::configs::init_for_tests();
    let config = get_config().unwrap();
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

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
    crate::configs::init_for_tests();
    let config = get_config().unwrap();
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

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
    crate::configs::init_for_tests();
    let config = get_config().unwrap();
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

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
    crate::configs::init_for_tests();
    let config = get_config().unwrap();
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

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
    crate::configs::init_for_tests();
    let config = get_config().unwrap();
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

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
    crate::configs::init_for_tests();
    let config = get_config().unwrap();
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

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
    crate::configs::init_for_tests();
    let config = get_config().unwrap();
    let fg = Color::from(&config.appearance.fg);
    let bg = Color::from(&config.appearance.bg);
    drop(config);

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
