/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */
use std::io::{Read, Write};

include!("src/app.rs");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    /*println!("cargo:rerun-if-changed=interfaces/fancy.xml");*/

    let mut data = String::new();
    std::fs::File::open("../interfaces/fancy.xml")
        .unwrap()
        .read_to_string(&mut data)?;
    let code = dbus_codegen::generate(
        &data,
        &dbus_codegen::GenOpts {
            methodtype: None,
            crhandler: None,
            ..Default::default()
        },
    )
    .unwrap();

    let mut file = std::fs::File::create("src/interfaces.rs").unwrap();
    file.write_all("#![allow(unused_imports)]\n".as_bytes())?;
    file.write_all(code.as_bytes())?;

    use clap::Shell;
    use std::str::FromStr;

    let outdir = std::env::var("OUT_DIR").unwrap();
    let mut app = get_app();
    for shell in Shell::variants() {
        app.gen_completions("fancy", Shell::from_str(shell).unwrap(), &outdir);
    }

    Ok(())
}
