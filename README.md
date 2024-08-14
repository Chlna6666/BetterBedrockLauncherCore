# 更好的我的世界基岩版启动器命令行版（BetterBedrockLauncherCore.exe ）



这是一个 Rust 编写的基岩版启动器核心



## 功能

- 多重版本启动

- 可支持教育版，教育预览版

- 基岩版，预览版

- 同一类只能开启一个（uwp是这样的

#### 使用方法

- 解压

```bash

$ ./BetterBedrockLauncherCore.exe unpack [源文件路径] [目标路径] [-f] [-dsign] [-dappx]



- [源文件路径]：要解压的应用程序包文件路径。

- [目标路径]：解压后内容要保存的目标路径。

- [-f]：是否强制替换已存在的文件。

- [-dsign]：是否删除签名文件。//不删除无法正常注册

- [-dappx]：是否删除源文件。

```

例子

```bash

 unpack c:/p/mc.appx d:/a -f -dsign -dappx

```

- 注册启动

```bash

$ ./BetterBedrockLauncherCore.exe   regpack [目标路径] [-start]

- [目标路径]：appx解压后的目标路径。

- [-start]：是否注册完成启动游戏。

```

例子

```bash

 regpack D:/Downloads/MC -start

```