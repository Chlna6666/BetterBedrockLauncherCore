use std::env;
use std::io;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use chrono::{DateTime, Utc, TimeZone, Local};


const CONFIG_FILE: &str = "config.json";
static CONFIG: once_cell::sync::Lazy<Mutex<Option<bool>>> = once_cell::sync::Lazy::new(|| Mutex::new(None));

const JSON_DATA: &str = r#"
    {
        "language": "en",
        "LogOutputFile":"true",
        "debug":"false"
    }
"#;

enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}


fn log(level: LogLevel, message: &str) { //todu检测debug输出和LogOutputFile
    let now: DateTime<Utc> = Utc::now();
    let local_now = now.with_timezone(&Local);

    let timestamp = local_now.format("%Y-%m-%d %H:%M:%S");

    let log_level_str = match level {
        LogLevel::Info => "INFO",
        LogLevel::Warning => "WARNING",
        LogLevel::Error => "ERROR",
        LogLevel::Debug => "DEBUG",
    };

    println!("[{}] [{}] {}", timestamp, log_level_str, message);

    // 写入文件
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs.log")
        .expect("Unable to open log file");

    writeln!(&mut file, "[{}] [{}] {}", timestamp, log_level_str, message)
        .expect("Unable to write to log file");
}


fn read_config_file() -> Result<bool, Box<dyn std::error::Error>> {
    // 使用 Mutex 来确保只读取一次配置
    let mut config_guard = CONFIG.lock().unwrap();

    if let Some(debug_value) = *config_guard {
        // 如果已经读取过，直接返回保存的值
        Ok(debug_value)
    } else {
        let path = Path::new(CONFIG_FILE);

        if !path.exists() {
            create_config_file()?;
        }

        let mut file = fs::File::open(path)?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        match serde_json::from_str::<serde_json::Value>(&contents) {
            Ok(config) => {
                let debug_value = config["debug"]
                    .as_str()
                    .map(|value| value.parse().unwrap_or(false))
                    .unwrap_or(false);

                // 保存配置值
                *config_guard = Some(debug_value);

                Ok(debug_value)
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}


fn create_config_file() -> Result<(), io::Error> {
    let mut file = fs::File::create(CONFIG_FILE)?;
    file.write_all(JSON_DATA.as_bytes())?;
    Ok(())
}



fn main() {
    /*log(LogLevel::Info, "This is an information message.");
    log(LogLevel::Warning, "This is a warning message.");
    log(LogLevel::Error, "This is an error message.");
    log(LogLevel::Debug, "This is a debug message.");*/
    // 检查命令行参数
    match read_config_file() {
        Ok(debug_enabled) => {
            if debug_enabled {
                log(LogLevel::Debug, "Debug 已启用。");
                log(LogLevel::Debug, "配置文件存在且格式正确。");
            }
        }
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
