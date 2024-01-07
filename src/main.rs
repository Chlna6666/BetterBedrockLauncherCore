use std::env;
use std::io;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use serde_json;

const CONFIG_FILE: &str = "config.json";

const JSON_DATA: &str = r#"
    {
        "language": "en"
    }
"#;

fn read_config_file() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(CONFIG_FILE);

    if !path.exists() {
        create_config_file()?;
    }

    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    match serde_json::from_str::<serde_json::Value>(&contents) {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}

fn create_config_file() -> Result<(), io::Error> {
    let mut file = fs::File::create(CONFIG_FILE)?;
    file.write_all(JSON_DATA.as_bytes())?;
    Ok(())
}


fn main() {
    // 检查命令行参数
    match read_config_file() {
        Ok(_) => println!("配置文件存在且格式正确。"),
        Err(err) => println!("读取或解析配置文件时出错：{:?}", err),
    }
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // 如果有命令行参数，则执行相应的操作
        let command = &args[1];
        match command.as_str() {
            "help" => {
                println!("执行帮助命令...");
                // 在这里执行帮助命令的逻辑
            }
            _ => {
                println!("未知命令，请输入有效命令或 'help' 获取帮助");
            }
        }
    } else {
        // 如果没有命令行参数，则进入交互模式
        println!("欢迎！输入命令或 'help' 获取帮助");
        loop {
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("无法读取输入");

            let trimmed_input = input.trim();
            match trimmed_input {
                "help" => {
                    println!("执行帮助命令...");
                    // 在这里执行帮助命令的逻辑
                }
                "exit" => {
                    println!("退出程序");
                    break;
                }
                _ => {
                    println!("未知命令，请输入有效命令或 'help' 获取帮助");
                }
            }
        }
    }
}
