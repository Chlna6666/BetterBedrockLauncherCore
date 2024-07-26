use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;


use tokio::main;
use windows::core::{Error, HSTRING, Result};
use windows::Foundation::{IAsyncOperationWithProgress, Uri};
use windows::Management::Deployment::{DeploymentOptions, DeploymentProgress, DeploymentResult, PackageManager, RemovalOptions};
use xml::reader::{EventReader, XmlEvent};
use zip::ZipArchive;
use BetterBedrockLauncherCore::{debug, error, info};


fn create_parent_directories(file_path: &Path) -> io::Result<()> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn extract_zip(file: File, destination_path: &str, force_replace: bool, delete_signature: bool) -> zip::result::ZipResult<()> {
    if let Err(e) = create_parent_directories(Path::new(destination_path)) {
        error!("无法创建父目录: {}", e);
        return Err(zip::result::ZipError::Io(e.into()));
    }
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_name = file.mangled_name();
        let output_file_path = Path::new(destination_path).join(&file_name);

        if output_file_path.exists() {
            if force_replace {
                info!("{} 已存在，将被替换", file_name.display());
                fs::remove_file(&output_file_path).map_err(|e| zip::result::ZipError::Io(e))?;
            } else {
                info!("{} 已存在，跳过", file_name.display());
                continue;
            }
        }

        info!("正在解压: {}", file_name.display());
        if let Err(e) = create_parent_directories(&output_file_path) {
            error!("无法创建父目录: {}", e);
            continue;
        }

        let mut output_file = File::create(&output_file_path)?;
        std::io::copy(&mut file, &mut output_file)?;
    }

    if delete_signature {
        let signature_path = PathBuf::from(destination_path).join("AppxSignature.p7x");
        if signature_path.exists() {
            fs::remove_file(&signature_path).map_err(|e| zip::result::ZipError::Io(e))?;
            info!("签名文件删除成功");
        }
    }

    Ok(())
}

async fn register_appx_package_async(appx_manifest_path: &str) -> Result<DeploymentResult> {
    info!("注册 APPX：{}", appx_manifest_path);
    let package_manager = PackageManager::new()?;
    let relative_uri = HSTRING::from(appx_manifest_path);
    let uri = Uri::CreateUri(&relative_uri)?;

    let result_async: IAsyncOperationWithProgress<DeploymentResult, DeploymentProgress> =
        package_manager.RegisterPackageAsync(&uri, None, DeploymentOptions::DevelopmentMode)?;

    let result: DeploymentResult = result_async.get()?;

    info!("APPX 注册成功");
    Ok(result)
}

async fn handle_remove_package_register_start(packagefullname: &str, manifest_path: &str, _package_path: &str, auto_start: bool, edition: &str) {
    let hstring_packagefullname = HSTRING::from(packagefullname);
    let package_manager = PackageManager::new().expect("无法创建 PackageManager");

    match package_manager.RemovePackageWithOptionsAsync(&hstring_packagefullname, RemovalOptions::PreserveApplicationData) {
        Ok(async_op) => {
            let async_result = async_op.get().expect("等待异步操作失败");

            if let Ok(error_text) = async_result.ErrorText() {
                error!("移除包失败。错误文本: {:?}", error_text);
            }

            if let Ok(extended_error_code) = async_result.ExtendedErrorCode() {
                error!("扩展错误代码: {:?}", extended_error_code);
            }

            if let Ok(is_registered) = async_result.IsRegistered() {
                if is_registered {
                    info!("包已成功移除");

                    match register_appx_package_async(manifest_path).await {
                        Ok(result) => {
                            info!("Appx 包成功注册: {:?}", result);
                        },
                        Err(err) => {
                            error!("注册 Appx 包失败: {}", err);
                        }
                    }

                    if auto_start {
                        match edition {
                            "Microsoft.MinecraftUWP" => start_minecraft_release(),
                            "Microsoft.MinecraftWindowsBeta" => start_minecraft_beta(),
                            "Microsoft.MinecraftEducationEdition" => start_minecraft_education(),
                            "Microsoft.MinecraftEducationPreview" => start_minecraft_education_preview(),
                            _ => Ok(()),
                        }.unwrap_or_else(|err| error!("启动失败: {}", err));
                    }
                } else {
                    error!("包移除失败");
                }
            }
        },
        Err(err) => {
            error!("调用 RemovePackageAsync 出错: {:?}", err);
        }
    }
}


async fn register_start(manifest_path: &str, auto_start: bool, edition: &str) {
    match register_appx_package_async(manifest_path).await {
        Ok(result) => {
            info!("Appx 包成功注册: {:?}", result);
        },
        Err(err) => {
            error!("注册 Appx 包失败: {}", err);
        }
    }

    if auto_start {
        match edition {
            "Microsoft.MinecraftUWP" => start_minecraft_release(),
            "Microsoft.MinecraftWindowsBeta" => start_minecraft_beta(),
            "Microsoft.MinecraftEducationEdition" => start_minecraft_education(),
            "Microsoft.MinecraftEducationPreview" => start_minecraft_education_preview(),
            _ => Ok(()),
        }.unwrap_or_else(|err| error!("启动失败: {}", err));
    }
}

fn start_minecraft_release() -> io::Result<()> {
    let output = Command::new("explorer.exe")
        .arg("shell:appsFolder\\Microsoft.MinecraftUWP_8wekyb3d8bbwe!App")
        .output()?;

    info!("{:?}", output);
    Ok(())
}

fn start_minecraft_beta() -> io::Result<()> {
    let output = Command::new("explorer.exe")
        .arg("shell:appsFolder\\Microsoft.MinecraftWindowsBeta_8wekyb3d8bbwe!App")
        .output()?;

    info!("{:?}", output);
    Ok(())
}

fn start_minecraft_education() -> io::Result<()> {
    let output = Command::new("explorer.exe")
        .arg("shell:appsFolder\\Microsoft.MinecraftEducationEdition_8wekyb3d8bbwe!Microsoft.MinecraftEducationEdition")
        .output()?;

    info!("{:?}", output);
    Ok(())
}

fn start_minecraft_education_preview() -> io::Result<()> {
    let output = Command::new("explorer.exe")
        .arg("shell:appsFolder\\Microsoft.MinecraftEducationPreview_8wekyb3d8bbwe!Microsoft.MinecraftEducationEdition")
        .output()?;

    info!("{:?}", output);
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
                        return Err(Error::from(io::Error::new(io::ErrorKind::Other, "无法获取包家族名称")));
                    };

                    let package_full_name = if let Ok(package_full_name) = package.Id().and_then(|id| id.FullName()) {
                        Some(package_full_name.to_string())
                    } else {
                        return Err(Error::from(io::Error::new(io::ErrorKind::Other, "无法获取包全名")));
                    };

                    Ok(Some((version.unwrap(), package_family_name.unwrap().to_string(), package_full_name.unwrap())))
                }
                Err(err) => Err(err.into()),
            }
        }
        Err(err) => Err(err.into()),
    }
}

#[main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let command = &args[1];
    match command.to_lowercase().as_str() {
        "help" => {
            println!("用法:");
            println!("  unpack <source_path> <destination_path> [-f] [-dsign] [-dappx]");
            println!("    解压指定的 appx 文件。");
            println!("    <source_path>: appx 文件的路径。");
            println!("    <destination_path>: 解压内容的目标目录。");
            println!("    -f: 强制替换已存在的文件。");
            println!("    -dsign: 解压后删除签名文件。");
            println!("    -dappx: 解压后删除源 appx 文件。");
            println!("    例子: unpack c:/p/mc.appx d:/a -f -dsign -dappx");
            println!();
            println!("  regpack <package_path> [-start]");
            println!("    注册指定路径的 appx 包。");
            println!("    <package_path>: 包含 AppxManifest.xml 的目录路径。");
            println!("    -start: 注册后自动启动应用。");
            println!("    例子: regpack D:/Downloads/MC -start");
            println!();
            println!("  help");
            println!("    显示此帮助信息。");
        }
        "unpack" => {
            if args.len() < 4 {
                println!("用法: unpack 所在文件目标路径 解压后的路径 [-f] [-dsign] [-dappx]");
                println!("例子: unpack c:/p/mc.appx d:/a -f -dsign -dappx");
            } else {
                let source_path = &args[2];
                let destination_path = &args[3];
                let force_replace = args.contains(&"-f".to_string());
                let delete_signature = args.contains(&"-dsign".to_string());
                let delete_source = args.contains(&"-dappx".to_string());

                info!("源文件路径: {}", source_path);
                info!("目标路径: {}", destination_path);
                info!("是否强制替换: {}", force_replace);
                info!("是否删除签名文件: {}", delete_signature);
                info!("是否删除源文件: {}", delete_source);

                if let Err(e) = fs::create_dir_all(destination_path) {
                    error!("无法创建目标路径: {}", e);
                    return;
                }

                let file = File::open(source_path).expect("无法打开源文件");

                match extract_zip(file, destination_path, force_replace, delete_signature) {
                    Ok(_) => {
                        info!("解压完成");
                        if delete_source {
                            if let Err(e) = fs::remove_file(source_path) {
                                error!("无法删除源文件: {}", e);
                            } else {
                                info!("源文件删除成功");
                            }
                        }
                    }
                    Err(err) => error!("解压出错: {:?}", err),
                }
            }
        }
        "regpack" => {
            if args.len() < 3 {
                println!("用法: regpack 路径 [-start]");
                println!("例子: regpack D:/Downloads/MC -start");
            } else {
                let package_path = &args[2];
                let auto_start = args.contains(&"-start".to_string());

                let manifest_path = format!("{}/AppxManifest.xml", package_path.replace("\\", "/"));
                let mut manifest = File::open(&manifest_path).expect("无法打开文件");

                let mut xml_data = String::new();
                manifest.read_to_string(&mut xml_data).expect("无法读取文件");

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

                match identity_name.as_deref() {
                    Some("Microsoft.MinecraftWindowsBeta") => handle_minecraft_beta(identity_version, &manifest_path, package_path, auto_start).await,
                    Some("Microsoft.MinecraftUWP") => handle_minecraft_uwp(identity_version, &manifest_path, package_path, auto_start).await,
                    Some("Microsoft.MinecraftEducationEdition") => handle_minecraft_education(identity_version, &manifest_path, package_path, auto_start).await,
                    Some("Microsoft.MinecraftEducationPreview") => handle_minecraft_education_preview(identity_version, &manifest_path, package_path, auto_start).await,
                    Some(name) => info!("未知包名: {}", name),
                    None => info!("未知包名"),
                }
            }
        }
        _ => {
            println!("未知命令，请输入有效命令或 'help' 获取帮助");
        }
    }
}

async fn handle_minecraft_beta(identity_version: Option<String>, manifest_path: &str, package_path: &str, auto_start: bool) {
    let app_user_model_id = "Microsoft.MinecraftWindowsBeta_8wekyb3d8bbwe!App";
    match get_package_info(app_user_model_id) {
        Ok(Some((version, package_family_name, package_full_name))) => {
            debug!("Version: {:?}", version);
            debug!("Package Family Name: {}", package_family_name);
            debug!("Package Full Name: {}", package_full_name);

            if let Some(identity_version_str) = identity_version {
                if version == identity_version_str {
                    debug!("版本匹配");
                    if auto_start {
                        if let Err(err) = start_minecraft_beta() {
                            error!("启动失败: {}", err);
                        }
                    }
                } else {
                    info!("版本不匹配");
                    handle_remove_package_register_start(&package_full_name, manifest_path, package_path, auto_start, "Microsoft.MinecraftWindowsBeta").await;
                }
            }
        }
        Ok(None) => {
            error!("无法获取包信息");
            register_start(manifest_path, auto_start, "Microsoft.MinecraftWindowsBeta").await;
        }
        Err(err) => {
            error!("{:?}", err);
            debug!("没有注册过 appx");
            register_start(manifest_path, auto_start, "Microsoft.MinecraftWindowsBeta").await;
        }
    }
}

async fn handle_minecraft_uwp(identity_version: Option<String>, manifest_path: &str, package_path: &str, auto_start: bool) {
    let app_user_model_id = "Microsoft.MinecraftUWP_8wekyb3d8bbwe!App";
    match get_package_info(app_user_model_id) {
        Ok(Some((version, package_family_name, package_full_name))) => {
            debug!("Version: {:?}", version);
            debug!("Package Family Name: {}", package_family_name);
            debug!("Package Full Name: {}", package_full_name);

            if let Some(identity_version_str) = identity_version {
                if version == identity_version_str {
                    debug!("版本匹配");
                    if auto_start {
                        if let Err(err) = start_minecraft_release() {
                            error!("启动失败: {}", err);
                        }
                    }
                } else {
                    debug!("版本不匹配");
                    handle_remove_package_register_start(&package_full_name, manifest_path, package_path, auto_start, "Microsoft.MinecraftUWP").await;
                }
            }
        }
        Ok(None) => {
            error!("无法获取包信息");
            register_start(manifest_path, auto_start, "Microsoft.MinecraftUWP").await;
        }
        Err(err) => {
            error!("{:?}", err);
            info!("没有注册过 appx");
            register_start(manifest_path, auto_start, "Microsoft.MinecraftUWP").await;
        }
    }
}

async fn handle_minecraft_education(identity_version: Option<String>, manifest_path: &str, package_path: &str, auto_start: bool) {
    let app_user_model_id = "Microsoft.MinecraftEducationEdition_8wekyb3d8bbwe!Microsoft.MinecraftEducationEdition";
    match get_package_info(app_user_model_id) {
        Ok(Some((version, package_family_name, package_full_name))) => {
            debug!("Version: {:?}", version);
            debug!("Package Family Name: {}", package_family_name);
            debug!("Package Full Name: {}", package_full_name);

            if let Some(identity_version_str) = identity_version {
                if version == identity_version_str {
                    debug!("版本匹配");
                    if auto_start {
                        if let Err(err) = start_minecraft_education() {
                            error!("启动失败: {}", err);
                        }
                    }
                } else {
                    debug!("版本不匹配");
                    handle_remove_package_register_start(&package_full_name, manifest_path, package_path, auto_start, "Microsoft.MinecraftEducationEdition").await;
                }
            }
        }
        Ok(None) => {
            error!("无法获取包信息");
            register_start(manifest_path, auto_start, "Microsoft.MinecraftEducationEdition").await;
        }
        Err(err) => {
            error!("{:?}", err);
            debug!("没有注册过 appx");
            register_start(manifest_path, auto_start, "Microsoft.MinecraftEducationEdition").await;
        }
    }
}

async fn handle_minecraft_education_preview(identity_version: Option<String>, manifest_path: &str, package_path: &str, auto_start: bool) {
    let app_user_model_id = "Microsoft.MinecraftEducationPreview_8wekyb3d8bbwe!Microsoft.MinecraftEducationEdition";
    match get_package_info(app_user_model_id) {
        Ok(Some((version, package_family_name, package_full_name))) => {
            debug!("Version: {:?}", version);
            debug!("Package Family Name: {}", package_family_name);
            debug!("Package Full Name: {}", package_full_name);

            if let Some(identity_version_str) = identity_version {
                if version == identity_version_str {
                    debug!("版本匹配");
                    if auto_start {
                        if let Err(err) = start_minecraft_education_preview() {
                            error!("启动失败: {}", err);
                        }
                    }
                } else {
                    debug!("版本不匹配");
                    handle_remove_package_register_start(&package_full_name, manifest_path, package_path, auto_start, "Microsoft.MinecraftEducationPreview").await;
                }
            }
        }
        Ok(None) => {
            error!("无法获取包信息");
            register_start(manifest_path, auto_start, "Microsoft.MinecraftEducationPreview").await;
        }
        Err(err) => {
            error!("{:?}", err);
            debug!("没有注册过 appx");
            register_start(manifest_path, auto_start, "Microsoft.MinecraftEducationPreview").await;
        }
    }
}
