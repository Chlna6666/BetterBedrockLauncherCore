use std::env;
use std::io;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use chrono::{DateTime, Utc, TimeZone, Local};



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
/*
    // 写入文件
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs.log")
        .expect("Unable to open log file");

    writeln!(&mut file, "[{}] [{}] {}", timestamp, log_level_str, message)
        .expect("Unable to write to log file");
 */
}



fn main() {
    let args: Vec<String> = std::env::args().collect();

    let command = &args[1];
    match command.to_lowercase().as_str() {
        "help" => {
            println!("执行帮助命令...");
            // 在这里执行帮助命令的逻辑
        }
        "unpack" => {
            if args.len() < 4 {
                println!("用法: unpack 所在文件目标路径 解压后的路径 [-f]");
                println!("例子: unpack c:/p/mc.appx d:/a -f");
            } else {
                let source_path = &args[2];
                let destination_path = &args[3];
                let force_replace = args.iter().any(|arg| arg == "-f");

                println!("源文件路径: {}", source_path);
                println!("目标路径: {}", destination_path);
                println!("是否强制替换: {}", force_replace);

                // 创建目标路径
                if let Err(e) = fs::create_dir_all(destination_path) {
                    println!("无法创建目标路径: {}", e);
                    return;
                }

                let mut command = Command::new("./7z.exe");
                command.arg("x").arg(source_path);

                // 如果指定了 -f 参数，则添加 -aoa 参数来强制替换文件
                if force_replace {
                    command.arg("-aoa");
                }

                let status = command
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .current_dir(destination_path)
                    .status()
                    .expect("无法执行解压命令");

                if status.success() {
                    println!("解压成功");
                } else {
                    println!("解压失败");
                }
            }
        }
        _ => {
            println!("未知命令，请输入有效命令或 'help' 获取帮助");
        }
    }
}
