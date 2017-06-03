// Copyright 2016-2017 the Tectonic Project
// Licensed under the MIT License.

#[macro_use] extern crate lazy_static;
extern crate tectonic;

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{Read, Result, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tectonic::engines::NoopIoEventBackend;
use tectonic::io::{FilesystemIo, IoStack, MemoryIo, try_open_file};
use tectonic::io::testing::SingleInputFileIo;
use tectonic::status::NoopStatusBackend;
use tectonic::TexEngine;

const TOP: &'static str = env!("CARGO_MANIFEST_DIR");


lazy_static! {
    static ref LOCK: Mutex<u8> = Mutex::new(0u8);
}


fn set_up_format_file(tests_dir: &Path) -> Result<SingleInputFileIo> {
    let mut fmt_path = tests_dir.to_owned();
    fmt_path.push("plain.fmt.gz");

    if try_open_file(&fmt_path).is_not_available() {
        // Well, we need to regenerate the format file. Not too difficult.
        let mut mem = MemoryIo::new(true);

        let mut plain_format_dir = tests_dir.to_owned();
        plain_format_dir.push("formats");
        plain_format_dir.push("plain");
        let mut fs = FilesystemIo::new(&plain_format_dir, false, false, HashSet::new());

        {
            let mut io = IoStack::new(vec![
                &mut mem,
                &mut fs,
            ]);

            let mut e = TexEngine::new();
            e.set_halt_on_error_mode(true);
            e.set_initex_mode(true);
            e.process(&mut io, &mut NoopIoEventBackend::new(),
                      &mut NoopStatusBackend::new(), "UNUSED.fmt.gz", "plain.tex")?;
        }

        let mut fmt_file = File::create(&fmt_path)?;
        fmt_file.write_all(mem.files.borrow().get(OsStr::new("plain.fmt.gz")).unwrap())?;
    }

    Ok(SingleInputFileIo::new(&fmt_path))
}


fn read_file<P: AsRef<Path>>(path: P) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut f = File::open(&path).unwrap();
    f.read_to_end(&mut buffer).unwrap();
    buffer
}

fn do_one(stem: &str, check_synctex: bool) {
    let _guard = LOCK.lock().unwrap(); // until we're thread-safe ...

    let mut p = PathBuf::from(TOP);
    p.push("tests");

    // IoProvider for the format file; with magic to generate the format
    // on-the-fly if needed.
    let mut fmt = set_up_format_file(&p).expect("couldn't write format file");

    // Ditto for the input file.
    p.push("tex-outputs");
    p.push(stem);
    p.set_extension("tex");
    let texname = p.file_name().unwrap().to_str().unwrap().to_owned();
    let mut tex = SingleInputFileIo::new(&p);

    // Read in the expected "log" output ...
    p.set_extension("log");
    let logname = p.file_name().unwrap().to_owned();
    let expected_log = read_file(&p);

    // ... and the expected XDVI output.
    p.set_extension("xdv");
    let xdvname = p.file_name().unwrap().to_owned();
    let expected_xdv = read_file(&p);


    // MemoryIo layer that will accept the outputs.
    let mut mem = MemoryIo::new(true);

    // Run the engine!
    {
        let mut io = IoStack::new(vec![
            &mut mem,
            &mut tex,
            &mut fmt,
        ]);
        let mut e = TexEngine::new();
        e.set_initex_mode(false); // TODO: this shouldn't be necessary
        e.process(&mut io, &mut NoopIoEventBackend::new(),
                  &mut NoopStatusBackend::new(), "plain.fmt.gz", &texname).unwrap();
    }

    // Check that log and xdv match expectations.

    let files = mem.files.borrow();

    let observed_log = files.get(&logname).unwrap();
    assert_eq!(&expected_log, observed_log);

    let observed_xdv = files.get(&xdvname).unwrap();
    assert_eq!(&expected_xdv, observed_xdv);

    if (check_synctex) {
        p.set_extension("synctex");
        let expected_synctex = read_file(&p);
        // synctex file is created in the current directory
        let synctex_file = PathBuf::from(stem).with_extension("synctex");
        let observed_synctex = read_file(&synctex_file);
        assert_eq!(expected_synctex, observed_synctex);
        fs::remove_file(&synctex_file).unwrap();
    }
}


// Keep these alphabetized.

#[test]
fn md5_of_hello() { do_one("md5_of_hello", false) }

#[test]
fn negative_roman_numeral() { do_one("negative_roman_numeral", false) }

#[test]
fn pdfoutput() { do_one("pdfoutput", false) }

#[test]
fn synctex() { do_one("synctex", true) }

#[test]
fn the_letter_a() { do_one("the_letter_a", false) }
