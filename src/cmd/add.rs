use colored::Colorize;
use std::path::Path;
use std::process::Command;

pub fn add(name: String) {
    if name.is_empty() {
        eprintln!("Name cannot be empty");
        return;
    }


    let plugin_path_str = format!("plugins/{name}");
    let plugin_path = Path::new(&plugin_path_str);


    //检测有没有这个插件
    match plugin_path.try_exists() {
        Ok(boo) => {
            if !boo {
                println!(
                    "{}{}",
                    "Plugin already exists at path: ".truecolor(234, 108, 108),
                    format!("{plugin_path_str}").truecolor(234, 108, 108)
                );
                return;
            }
        }
        Err(e) => {
            println!("{e}");
            return;
        }
    }

    let mut cargo_command = Command::new("cargo");
    cargo_command.arg("add").arg("--path").arg(plugin_path);

    match cargo_command.status() {
        Ok(status) if status.success() => {
            println!(
                "\n{}",
                format!("Plugin '{}' add successfully!", name).truecolor(202, 225, 205),
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
