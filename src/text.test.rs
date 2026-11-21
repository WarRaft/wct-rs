use super::*;
use crate::tests::file;

#[test]
fn text_reads_back_to_the_same_bytes() {
    let f = file();
    let text = to_text(&f, Script::Jass).unwrap();
    assert_eq!(
        text,
        "// @wct version=1\n// @wct comment \"Helpers, \\\"shared\\\"\\r\\nby me\"\n// @wct header\nfunction Helper takes nothing returns nothing\nendfunction\n\n// @wct trigger\nfunction Trig_A_Actions takes nothing returns nothing\n    call Helper()\nendfunction\n// @wct trigger none\n// @wct trigger\n\n"
    );
    assert_eq!(from_text(&text).unwrap(), f);
    // As the editor may give it back, with `\r\n`.
    assert_eq!(from_text(&text.replace('\n', "\r\n")).unwrap(), f);

    let lua = to_text(&f, Script::Lua).unwrap();
    assert!(lua.starts_with("-- @wct version=1\n"));
    assert_eq!(from_text(&lua).unwrap(), f);

    // `\n` code stays `\n`; the 1.31 layout says so.
    let mut g = f.clone();
    g.magic = true;
    g.header = Code::new(b"a\nb\n");
    g.triggers = vec![Code::new(b"c")];
    let text = to_text(&g, Script::Jass).unwrap();
    assert!(text.starts_with("// @wct version=1\n// @wct magic\n// @wct eol=lf\n"), "{text}");
    assert_eq!(from_text(&text).unwrap(), g);

    // Version 0: no comment, no header.
    let v0 = CustomText { version: 0, triggers: vec![Code(None)], ..CustomText::default() };
    let text = to_text(&v0, Script::Jass).unwrap();
    assert_eq!(text, "// @wct version=0\n// @wct trigger none\n");
    assert_eq!(from_text(&text).unwrap(), v0);
}

#[test]
fn errors_in_the_text() {
    let errors = |t: &str| -> Vec<(usize, String)> {
        from_text(t).unwrap_err().into_iter().map(|e| (e.line, e.message)).collect()
    };
    let e = errors("call X()\n// @wct version=7\n// @wct trigger x\n// @wct trigger none\n// @wct comment \"late\"\n// @wct bogus\n");
    assert_eq!(e.len(), 5, "{e:?}");
    assert_eq!(e.iter().map(|e| e.0).collect::<Vec<_>>(), [1, 2, 3, 5, 6]);
    assert!(e[0].1.starts_with("code goes after"), "{e:?}");
    assert!(e[1].1.starts_with("unknown version 7"), "{e:?}");
    assert!(e[2].1.contains("trigger none"), "{e:?}");
    assert!(e[3].1.contains("isn't a marker here"), "{e:?}");
    assert!(e[4].1.contains("isn't a marker here"), "{e:?}");
    let e = errors("// @wct version=0\n// @wct header\nx\n");
    assert_eq!(e, [(1, "version 0 has no comment and no header".to_string())]);
    assert!(errors("// @wct comment unquoted\n")[0].1.contains("quoted"));
    // A line of code that reads as a marker can't be shown.
    let mut f = file();
    f.triggers = vec![Code::new(b"   //  @wct trigger")];
    assert!(to_text(&f, Script::Jass).unwrap_err().contains("marker"));
    assert!(!is_marker("// @wctx") && is_marker("-- @wct") && !is_marker("x // @wct"));
}
