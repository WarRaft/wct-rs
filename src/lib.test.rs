use super::*;

/// A version 1 file: a header, a trigger with code, one without, an
/// empty one.
pub(crate) fn file() -> CustomText {
    CustomText {
        magic: false,
        version: 1,
        comment: b"Helpers, \"shared\"\r\nby me".to_vec(),
        header: Code::new(b"function Helper takes nothing returns nothing\r\nendfunction\r\n"),
        triggers: vec![
            Code::new(b"function Trig_A_Actions takes nothing returns nothing\r\n    call Helper()\r\nendfunction"),
            Code(None),
            Code::new(b""),
        ],
    }
}

#[test]
fn reads_and_writes_back_every_layout() {
    for (magic, version) in [(false, 0), (false, 1), (true, 1), (true, 0)] {
        let mut f = file();
        f.magic = magic;
        f.version = version;
        if version == 0 {
            f.comment.clear();
            f.header = Code::default();
        }
        let bytes = f.to_bytes().unwrap();
        assert_eq!(bytes[..4] == MAGIC.to_le_bytes(), magic);
        assert_eq!(parse(&bytes).unwrap(), f, "{magic} {version}");
    }
    // Before 1.31 a count; from 1.31 the codes run to the end.
    let mut f = file();
    let classic = f.to_bytes().unwrap();
    f.magic = true;
    // The magic takes the count's 4 bytes.
    assert_eq!(f.to_bytes().unwrap().len(), classic.len());
    assert_eq!(file().header.text(), b"function Helper takes nothing returns nothing\r\nendfunction\r\n");
}

#[test]
fn bad_files() {
    let err = |b: &[u8]| parse(b).unwrap_err().to_string();
    assert!(err(&[2, 0, 0, 0]).contains("unknown wct version 2"));
    assert!(err(&[0, 0, 0, 0, 1, 0, 0, 0, 9, 0, 0, 0, b'a']).contains("code of 9 bytes"));
    let mut bytes = file().to_bytes().unwrap();
    bytes.push(0);
    assert!(err(&bytes).contains("extra byte"));
}

/// Every map's custom script reads and writes back to the same bytes,
/// and through the text form: `WCT_CORPUS` is a folder of
/// `<map>/war3map.wct`.
#[test]
#[ignore]
fn corpus() {
    let dir = std::env::var("WCT_CORPUS").expect("WCT_CORPUS");
    let (mut n, mut code, mut bad) = (0, 0, Vec::new());
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path().join("war3map.wct");
        let Ok(bytes) = std::fs::read(&path) else { continue };
        n += 1;
        match parse(&bytes) {
            Ok(f) if f.to_bytes().unwrap() != bytes => bad.push(format!("{}: differs", path.display())),
            Ok(f) => {
                code += f.triggers.iter().filter(|c| !c.text().is_empty()).count();
                let back = text::from_text(&text::to_text(&f, text::Script::Jass).unwrap());
                if back.as_ref().map(|f| f.to_bytes().unwrap()).ok() != Some(bytes.clone()) {
                    bad.push(format!("{}: differs through text: {:?}", path.display(), back.err()));
                }
            }
            Err(e) => bad.push(format!("{}: {e}", path.display())),
        }
    }
    println!("{n} files, {code} triggers with code");
    assert!(bad.is_empty(), "{} of {n}:\n{}", bad.len(), bad.join("\n"));
}
