use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};


use chrono::{DateTime, Local, Utc};
use std::io::prelude::*;
use zip::{ZipArchive, result::ZipError};
use windows::Management::Deployment::{PackageManager, DeploymentOptions, DeploymentResult, DeploymentProgress,};
use windows::Foundation::{IAsyncOperationWithProgress, Uri};
use windows::core::{HSTRING, Result};
use windows::Foundation::Collections::IIterable;

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


fn extract_zip(file: File, destination_path: &str, delete_signature: bool, force_replace: bool) -> zip::result::ZipResult<()> {
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let file_name = file.sanitized_name();

        let output_file_path = Path::new(destination_path).join(&file_name);

        // Check if the file already exists and if force_replace is true
        if force_replace && output_file_path.exists() {
            println!("{} 已存在，将被替换", file_name.display());
            fs::remove_file(&output_file_path).map_err(|e| zip::result::ZipError::Io(e))?;
        } else if output_file_path.exists() {
            println!("{} 已存在，跳过解压", file_name.display());
            continue;
        }

        let mut output_file = File::create(&output_file_path)?;

        std::io::copy(&mut file, &mut output_file)?;
    }

    if delete_signature {
        let signature_path = PathBuf::from(destination_path).join("AppxSignature.p7x");
        if signature_path.exists() {
            fs::remove_file(&signature_path).map_err(|e| zip::result::ZipError::Io(e))?;
            println!("签名文件删除成功");
        }
    }

    Ok(())
}





#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let command = &args[1];
    match command.to_lowercase().as_str() {
        "help" => {
            println!("执行帮助命令...");
            // 在这里执行帮助命令的逻辑
        }

        "unpack" => {
            if args.len() < 4 {
                println!("用法: unpack 所在文件目标路径 解压后的路径 [-f] [-dsign] [-dappx]");
                println!("例子: unpack c:/p/mc.appx d:/a -f -dsign -dappx");
            } else {
                let mut source_path = "";
                let mut destination_path = "";
                let mut force_replace = false;
                let mut delete_signature = false;
                let mut delete_source = false;

                for (i, arg) in args.iter().enumerate() {
                    match arg.as_str() {
                        "-f" => force_replace = true,
                        "-dsign" => delete_signature = true,
                        "-dappx" => delete_source = true,
                        _ => {
                            if i == 2 {
                                source_path = arg;
                            } else if i == 3 {
                                destination_path = arg;
                            }
                        }
                    }
                }

                println!("源文件路径: {}", source_path);
                println!("目标路径: {}", destination_path);
                println!("是否强制替换: {}", force_replace);
                println!("是否删除签名文件: {}", delete_signature);
                println!("是否删除源文件: {}", delete_source);

                // 创建目标路径
                if let Err(e) = fs::create_dir_all(destination_path) {
                    println!("无法创建目标路径: {}", e);
                    return;
                }

                // 打开源文件
                let file = File::open(source_path).expect("无法打开源文件");

                // 调用 extract_zip 函数
                match extract_zip(file, destination_path, force_replace, delete_signature) {
                    Ok(_) => {
                        println!("解压完成");
                        if delete_source {
                            if let Err(e) = fs::remove_file(source_path) {
                                println!("无法删除源文件: {}", e);
                            } else {
                                println!("源文件删除成功");
                            }
                        }
                    }
                    Err(err) => println!("解压出错: {:?}", err),
                }
            }
        }
        "regpack" => {
            if args.len() < 3 {
                println!("用法: regpack 路径 [-s]");
                println!("例子: regpack D:/Downloads/MC -s");
            } else {
                let mut package_path = "";
                let mut auto_start = false;

                for (i, arg) in args.iter().enumerate() {
                    match arg.as_str() {
                        "-s" => auto_start = true,
                        _ => {
                            if i == 2 {
                                package_path = arg;
                            }
                        }
                    }
                }

                println!("Appx 包路径: {}", package_path);
                println!("是否自动启动: {}", auto_start);




            }
        }
        _ => {
            println!("未知命令，请输入有效命令或 'help' 获取帮助");
        }
    }
}