//! `wct-rs`: checks custom script files and converts them to text and
//! back. See `usage`.

use std::process::ExitCode;

use wct_rs::{parse, text};

fn usage() {
    eprintln!("usage: wct-rs <war3map.wct>                         check: read and write back");
    eprintln!("       wct-rs --text [--lua] <war3map.wct>          the text form, to stdout");
    eprintln!("       wct-rs --from-text <in.j> <war3map.wct>      the text form back to a file");
}

fn run(args: &[&str]) -> Result<(), String> {
    let read = |path: &str| -> Result<(Vec<u8>, wct_rs::CustomText), String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        let file = parse(&bytes).map_err(|e| format!("{path}: {e}"))?;
        Ok((bytes, file))
    };
    match args {
        ["--text", rest @ ..] if matches!(rest, [_] | ["--lua", _]) => {
            let (script, path) = match rest {
                ["--lua", p] => (text::Script::Lua, *p),
                [p] => (text::Script::Jass, *p),
                _ => unreachable!("matched above"),
            };
            let (_, file) = read(path)?;
            print!("{}", text::to_text(&file, script).map_err(|e| format!("{path}: {e}"))?);
            Ok(())
        }
        ["--from-text", input, output] => {
            let src = std::fs::read_to_string(input).map_err(|e| format!("{input}: {e}"))?;
            let file = text::from_text(&src).map_err(|errors| {
                errors
                    .iter()
                    .map(|e| format!("{input}:{}: {}", e.line, e.message))
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
            let bytes = file.to_bytes().map_err(|e| format!("{input}: {e}"))?;
            std::fs::write(output, bytes).map_err(|e| format!("{output}: {e}"))
        }
        [path] if !path.starts_with('-') => {
            let (bytes, file) = read(path)?;
            if file.to_bytes().ok() != Some(bytes) {
                return Err(format!("{path}: differs after writing"));
            }
            let code = file.triggers.iter().filter(|c| !c.text().is_empty()).count();
            println!(
                "{path}: version {}, {} triggers, {code} with code, header {} bytes",
                file.version,
                file.triggers.len(),
                file.header.text().len()
            );
            Ok(())
        }
        _ => {
            usage();
            Err(String::new())
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}");
            }
            ExitCode::FAILURE
        }
    }
}
