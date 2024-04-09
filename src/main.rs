use std::{fs, io};
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use chrono::{DateTime, Local, Utc};
use windows::core::{Error, HSTRING, Result};
use windows::Foundation::Uri;
use windows::Management::Deployment::{DeploymentOptions, DeploymentResult, PackageManager, RemovalOptions};
use xml::reader::{EventReader, XmlEvent};
use zip::ZipArchive;
use crate::LogLevel::Info;

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


fn create_parent_directories(file_path: &Path) -> Result<()> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn extract_zip(file: File, destination_path: &str, force_replace: bool, delete_signature: bool) -> zip::result::ZipResult<()> {
    // 创建目标路径的父目录
    if let Err(e) = create_parent_directories(Path::new(destination_path)) {
        log(LogLevel::Info, &format!("无法创建父目录: {}", e));
        return Err(zip::result::ZipError::Io(e.into())); // 将 windows_result::error::Error 转换为 std::io::Error
    }
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let file_name = file.sanitized_name();

        let output_file_path = Path::new(destination_path).join(&file_name);

        // Check if the file already exists
        if output_file_path.exists() {
            if force_replace {
                log(LogLevel::Info, &format!("{} 已存在，将被替换", file_name.display()));
                fs::remove_file(&output_file_path).map_err(|e| zip::result::ZipError::Io(e))?;
            } else {
                log(LogLevel::Info, &format!("{} 已存在，将被替换", file_name.display()));
                continue;
            }
        }

        log(LogLevel::Info, &format!("正在解压: {}", file_name.display()));

        // 创建父目录
        if let Err(e) = create_parent_directories(&output_file_path) {
            log(LogLevel::Info, &format!("无法创建父目录: {}", e));
            continue;
        }

        // 创建新文件
        let mut output_file = File::create(&output_file_path)?;

        // 将压缩文件的内容复制到新文件中
        std::io::copy(&mut file, &mut output_file)?;
    }

    if delete_signature {
        let signature_path = PathBuf::from(destination_path).join("AppxSignature.p7x");
        if signature_path.exists() {
            fs::remove_file(&signature_path).map_err(|e| zip::result::ZipError::Io(e))?;
            log(LogLevel::Info, "签名文件删除成功");
        }
    }

    Ok(())
}

async fn register_appx_package_async(appx_manifest_path: &str) -> Result<DeploymentResult> {
    log(LogLevel::Info, &format!("注册 APPX：{}", appx_manifest_path));
    // 创建 PackageManager 实例
    let package_manager = PackageManager::new()?;

    // 将相对 URI 转换为 HSTRING
    let relative_uri = HSTRING::from(appx_manifest_path);
    log(LogLevel::Info, &format!("注册 arelative_uri：{}", relative_uri));
    let uri = Uri::CreateUri(&relative_uri)?;
    log(LogLevel::Info, &format!("注册 uri：{:?}", uri));

    // 使用基础 URI 注册包
    let result_async = package_manager.RegisterPackageAsync(&uri, None, DeploymentOptions::DevelopmentMode)?;
    let result = result_async.await?;

    log(LogLevel::Info, "APPX 注册成功");
    Ok(result)
}

async fn handle_remove_package_register_start(packagefullname: &str, manifest_path: &str,package_path:&str,auto_start: bool,edition:&str,) {
    let hstring_packagefullname = HSTRING::from(packagefullname);
    let package_manager = PackageManager::new().expect("Failed to create PackageManager");

    match package_manager.RemovePackageWithOptionsAsync(&hstring_packagefullname, RemovalOptions::PreserveApplicationData) {
        Ok(async_op) => {
            let async_result = async_op.await.expect("Failed to await async operation");

            if let Ok(error_text) = async_result.ErrorText() {
                log(LogLevel::Error, &format!("移除包失败。错误文本: {:?}", error_text));
            }

            if let Ok(extended_error_code) = async_result.ExtendedErrorCode() {
                log(LogLevel::Error, &format!("扩展错误代码: {:?}", extended_error_code));
            }

            if let Ok(is_registered) = async_result.IsRegistered() {
                if is_registered {
                    log(LogLevel::Info, "包已成功移除");

                    match register_appx_package_async(&manifest_path).await {
                        Ok(result) => {
                            log(LogLevel::Info, &format!("Appx包成功注册: {:?}", result));
                        },
                        Err(err) => {
                            log(LogLevel::Error, &format!("注册Appx包失败: {}", err));
                        }
                    }

                    if auto_start {
                        match edition {
                            "Microsoft.MinecraftUWP" => {
                                start_minecraft_release();
                            },
                            "Microsoft.MinecraftWindowsBeta" =>{
                                start_minecraft_beta();
                            },
                            "Microsoft.MinecraftEducationEdition" =>{
                                start_minecraft_education();
                            },
                            "Microsoft.MinecraftEducationPreview" =>{
                                start_minecraft_education_preview();
                            },


                            &_ => {}
                        }
                    }
                } else {
                    log(LogLevel::Error, "包移除失败");
                }
            }
        },
        Err(err) => {
            log(LogLevel::Error, &format!("Error calling RemovePackageAsync: {:?}", err));
        }
    }
}

async fn register_start(manifest_path: &str,auto_start: bool,edition:&str) {
    match register_appx_package_async(&manifest_path).await {
        Ok(result) => {
            log(LogLevel::Info, &format!("Appx包成功注册: {:?}", result));
        },
        Err(err) => {
            log(LogLevel::Error, &format!("注册Appx包失败: {}", err));
        }
    }
    if auto_start {
        match edition {
            "Microsoft.MinecraftUWP" => {
                start_minecraft_release();
            },
            "Microsoft.MinecraftWindowsBeta" =>{
                start_minecraft_beta();
            },
            "Microsoft.MinecraftEducationEdition" =>{
                start_minecraft_education();
            },
            "Microsoft.MinecraftEducationPreview" =>{
                start_minecraft_education_preview();
            },


            &_ => {}
        }
    }
}

fn start_minecraft_release() -> io::Result<()> {
    let output = Command::new("explorer.exe")
        .arg("shell:appsFolder\\Microsoft.MinecraftUWP_8wekyb3d8bbwe!App")
        .output()?;

    println!("Output: {:?}", output);
    Ok(())
}
fn start_minecraft_beta() -> io::Result<()> {
    let output = Command::new("explorer.exe")
        .arg("shell:appsFolder\\Microsoft.MinecraftWindowsBeta_8wekyb3d8bbwe!App")
        .output()?;

    println!("Output: {:?}", output);
    Ok(())
}

/*
看了一眼AppxManifest.xml Application Id="Microsoft.MinecraftEducationEdition"
我说怎么无法获取信息&启动 浪费几天时间 我真↑服了 start_minecraft_education_exe 就不删了
 */
fn start_minecraft_education_exe(appx_manifest_path: &str) -> io::Result<()> {
    println!("m1133 {:?}", appx_manifest_path);

    let mut minecraft_exe_path = PathBuf::from(appx_manifest_path.replace("\\", "/")); //能用就行
    minecraft_exe_path.push("Minecraft.Windows.exe");
    println!("minecraft_exe_path {:?}", minecraft_exe_path);

    // 检查文件是否存在
    if !minecraft_exe_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Minecraft.Windows.exe not found",
        ));
    }

    let result = Command::new(minecraft_exe_path)
        .spawn();

    match result {
        Ok(child) => {
            println!("Minecraft Education Edition started with PID: {:?}", child.id());
        },
        Err(e) => {
            println!("Failed to start Minecraft Education Edition: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
fn start_minecraft_education() -> io::Result<()> {
    let output = Command::new("explorer.exe")
        .arg("shell:appsFolder\\Microsoft.MinecraftEducationEdition_8wekyb3d8bbwe!Microsoft.MinecraftEducationEdition")
        .output()?;

    println!("Output: {:?}", output);
    Ok(())
}
fn start_minecraft_education_preview() -> io::Result<()> {
    let output = Command::new("explorer.exe")
        .arg("shell:appsFolder\\Microsoft.MinecraftEducationPreview_8wekyb3d8bbwe!Microsoft.MinecraftEducationEdition")
        .output()?;

    println!("Output: {:?}", output);
    Ok(())
}

fn get_package_info(app_user_model_id: &str) -> Result<Option<(String, String, String)>> {
    match windows::ApplicationModel::AppInfo::GetFromAppUserModelId(&app_user_model_id.into()) {
        Ok(app_info) => {
            match app_info.Package() {
                Ok(package) => {
                    let version = if let Ok(version) = package.Id().and_then(|id| id.Version()) {
                        Some(format!("{}.{}.{}.{}", version.Major, version.Minor, version.Build, version.Revision))
                    } else {
                        None
                    };

                    let package_family_name = if let Ok(package_family_name) = package.Id().and_then(|id| id.FamilyName()) {
                        Some(package_family_name)
                    } else {
                        return Err(Error::from(io::Error::new(io::ErrorKind::Other, "Failed to get package family name")));
                    };

                    let package_full_name = if let Ok(package_full_name) = package.Id().and_then(|id| id.FullName()) {
                        Some(package_full_name.to_string())
                    } else {
                        return Err(Error::from(io::Error::new(io::ErrorKind::Other, "Failed to get package full name")));
                    };

                    Ok(Some((version.unwrap(), package_family_name.unwrap().to_string(), package_full_name.unwrap())))
                }
                Err(err) => Err(err.into()),
            }
        }
        Err(err) => Err(err.into()),
    }
}



struct HString(&'static str);

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let command = &args[1];
    match command.to_lowercase().as_str() {
        "help" => {
            println!("114514");

        }

        "unpack" => {

            if args.len() < 4 {
                println!("用法: unpack 所在文件目标路径 解压后的路径 [-f] [-dsign] [-dappx]");
                println!("例子: unpack c:/p/mc.appx d:/a -f - -dappx");
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

                log(LogLevel::Info, &format!("源文件路径: {}", source_path));
                log(LogLevel::Info, &format!("目标路径: {}", destination_path));
                log(LogLevel::Info, &format!("是否强制替换: {}", force_replace));
                log(LogLevel::Info, &format!("是否删除签名文件: {}", delete_signature));
                log(LogLevel::Info, &format!("是否删除源文件: {}", delete_source));


                // 创建目标路径
                if let Err(e) = fs::create_dir_all(destination_path) {
                    log(LogLevel::Info, &format!("无法创建目标路径: {}", e));
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
                    println!("用法: regpack 路径 [-start]");
                    println!("例子: regpack D:/Downloads/MC -start");
                } else {
                    let mut package_path = "";
                    let mut auto_start = false;

                    for (i, arg) in args.iter().enumerate() {
                        match arg.as_str() {
                            "-start" => auto_start = true,
                            _ => {
                                if i == 2 {
                                    package_path = arg;
                                }
                            }
                        }
                    }

                    let manifest_path = format!("{}{}",package_path.replace("\\", "/"), "/AppxManifest.xml");
                    let mut manifest = File::open(manifest_path.clone()).expect("Unable to open file");

                    let mut xml_data = String::new();
                    manifest.read_to_string(&mut xml_data).expect("Unable to read file");

                    let parser = EventReader::from_str(&xml_data);
                    let mut identity_name = None;
                    let mut identity_version = None;

                    for e in parser {
                        match e {
                            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                                if name.local_name == "Identity" {
                                    for attr in attributes {
                                        if attr.name.local_name == "Name" {
                                            identity_name = Some(attr.value);
                                        } else if attr.name.local_name == "Version" {
                                            identity_version = Some(attr.value);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    match identity_name {
                        Some(name) if name == "Microsoft.MinecraftWindowsBeta" => {

                            let app_user_model_id = "Microsoft.MinecraftWindowsBeta_8wekyb3d8bbwe!App";

                            match get_package_info(&app_user_model_id) {
                                Ok(Some((version, package_family_name,package_full_name))) => {
                                    log(LogLevel::Debug, &format!("Version:  {:?}",version));
                                    log(LogLevel::Debug, &format!("Package Family Name: {}", package_family_name));
                                    log(LogLevel::Debug, &format!("package_full_name: {}", package_full_name));


                                    if let Some(identity_version_str) = identity_version {
                                        if version == identity_version_str {
                                            println!("版本匹配");
                                            if auto_start{
                                                start_minecraft_beta();
                                            }

                                        } else {
                                            println!("版本不匹配");
                                            handle_remove_package_register_start(&*package_full_name, &*manifest_path, &package_path.to_string(), auto_start, &*name).await;

                                        }
                                    }

                                }
                                Ok(None) => {
                                    println!("Failed to get package info");
                                    register_start(&*manifest_path, auto_start, &*name).await;
                                }
                                Err(err) => {
                                    println!("Error: {:?}", err);
                                    register_start(&*manifest_path, auto_start, &*name).await;
                                }
                            }

                        }
                        Some(name) if name == "Microsoft.MinecraftUWP" => {
                            let app_user_model_id = "Microsoft.MinecraftUWP_8wekyb3d8bbwe!App";

                            match get_package_info(&app_user_model_id) {
                                Ok(Some((version, package_family_name,package_full_name))) => {
                                    log(LogLevel::Debug, &format!("Version:  {:?}",version));
                                    log(LogLevel::Debug, &format!("Package Family Name: {}", package_family_name));
                                    log(LogLevel::Debug, &format!("package_full_name: {}", package_full_name));

                                    if let Some(identity_version_str) = identity_version {
                                        if version == identity_version_str {
                                            println!("版本匹配");

                                            if auto_start {
                                                start_minecraft_release();
                                            }
                                        } else {
                                            println!("版本不匹配");

                                            handle_remove_package_register_start(&*package_full_name, &*manifest_path, &package_path.to_string(), auto_start, &*name).await;

                                        }
                                    }
                                }
                                Ok(None) => {
                                    println!("Failed to get package info");
                                    register_start(&*manifest_path, auto_start, &*name).await;
                                }
                                Err(err) => {
                                    println!("Error: {:?}", err);
                                    register_start(&*manifest_path, auto_start, &*name).await;
                                }
                            }
                        }
                        Some(name) if name == "Microsoft.MinecraftEducationEdition" => {
                            let app_user_model_id = "Microsoft.MinecraftEducationEdition_8wekyb3d8bbwe!Microsoft.MinecraftEducationEdition";
                            match get_package_info(&app_user_model_id) {
                                Ok(Some((version, package_family_name,package_full_name))) => {
                                    log(LogLevel::Debug, &format!("Version:  {:?}",version));
                                    log(LogLevel::Debug, &format!("Package Family Name: {}", package_family_name));
                                    log(LogLevel::Debug, &format!("package_full_name: {}", package_full_name));

                                    if let Some(identity_version_str) = identity_version {
                                        if version == identity_version_str {
                                            println!("版本匹配");

                                            if auto_start {
                                           start_minecraft_education();

                                            }
                                        } else {
                                            println!("版本不匹配");
                                            handle_remove_package_register_start(&*package_full_name, &*manifest_path, &package_path.to_string(), auto_start, &*name).await;

                                        }
                                    }
                                }
                                Ok(None) => {
                                    println!("Failed to get package info");
                                    register_start(&*manifest_path, auto_start, &*name).await;
                                }
                                Err(err) => {
                                    println!("Error: {:?}", err);
                                    register_start(&*manifest_path, auto_start, &*name).await;
                                }
                            }

                        }
                        Some(name) if name == "Microsoft.MinecraftEducationPreview" => {

                            let app_user_model_id = "Microsoft.MinecraftEducationPreview_8wekyb3d8bbwe!Microsoft.MinecraftEducationEdition";

                            match get_package_info(&app_user_model_id) {
                                Ok(Some((version, package_family_name,package_full_name))) => {
                                    log(LogLevel::Debug, &format!("Version:  {:?}",version));
                                    log(LogLevel::Debug, &format!("Package Family Name: {}", package_family_name));
                                    log(LogLevel::Debug, &format!("package_full_name: {}", package_full_name));

                                    if let Some(identity_version_str) = identity_version {
                                        if version == identity_version_str {
                                            println!("版本匹配");

                                            if auto_start {
                                               start_minecraft_education_preview();
                                            }
                                        } else {
                                            println!("版本不匹配");
                                            handle_remove_package_register_start(&*package_full_name, &*manifest_path, &package_path.to_string(), auto_start, &*name).await;

                                        }
                                    }
                                }
                                Ok(None) => {
                                    println!("Failed to get package info");
                                    register_start(&*manifest_path, auto_start, &*name).await;
                                }
                                Err(err) => {
                                    println!("Error: {:?}", err);
                                    register_start(&*manifest_path, auto_start, &*name).await;
                                }
                            }
                        }

                        Some(name) => {
                            println!("未知包名: {}", name);
                        }
                        None => {
                            println!("未知包名");

                        }
                    }
                }
            }
            _ => {
                println!("未知命令，请输入有效命令或 'help' 获取帮助");
            }
    }
}