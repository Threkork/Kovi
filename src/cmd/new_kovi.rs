use crate::cmd::{get_latest_version, DEFAULT_MAIN_CODE};
use colored::Colorize;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub fn new_kovi(name: String, version: Option<String>) {
    let mut cargo_command = Command::new("cargo");
    cargo_command.arg("new").arg(&name);

    match cargo_command.status() {
        Ok(status) if status.success() => {
            let path = format!("./{}", name);
            let path = Path::new(&path);

            let cargo_path = path.join("Cargo.toml");

            //给Cargo.toml加入Kovi依赖
            let mut cargo_toml = std::fs::OpenOptions::new()
                .append(true)
                .open(cargo_path)
                .expect("Failed to open Cargo.toml");

            let version = match version {
                Some(v) => v,
                None => match get_latest_version() {
                    Ok(v) => v,
                    Err(e) => {
                        let v = env!("CARGO_PKG_VERSION").to_string();
                        //报错获取失败，使用默认版本
                        eprintln!(
                            "Failed to get latest version: {}\nUse default version: {}",
                            e, v
                        );
                        v
                    }
                },
            };

            writeln!(
                cargo_toml,
                "kovi = {{ version = \"{}\", features = [\"logger\"] }}",
                version
            )
            .expect("Failed to write to Cargo.toml");

            // writeln!(cargo_toml, "kovi = {{ path = \"../../kovi\" }}")
            //     .expect("Failed to write to Cargo.toml");

            writeln!(cargo_toml, "\n[workspace]").expect("Failed to write to Cargo.toml");
            // 清空src/main.rs，然后传入默认的代码
            let main_path = path.join("src/main.rs");
            let mut main_rs = std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(main_path)
                .expect("Failed to open lib.rs");


            main_rs
                .write_all(DEFAULT_MAIN_CODE.as_bytes())
                .expect("Failed to write to lib.rs");


            println!(
                "\n{}\n{}",
                format!("KoviBot '{}' created successfully!", name).truecolor(202, 225, 205),
                format!("You can: \ncd ./{};\nkovi create <NAME>", name),
            );
        }
        Ok(status) => {
            eprintln!("Cargo exited with status: {}", status);
        }
        Err(e) => {
            eprintln!("Failed to execute cargo: {}", e);
        }
    }
}
