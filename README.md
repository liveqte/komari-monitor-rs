# Komari-Monitor-rs

![](https://hitscounter.dev/api/hit?url=https%3A%2F%2Fgithub.com%2Frsbench%2Frsbench&label=&icon=github&color=%23160d27)
![komari-monitor-rs](https://socialify.git.ci/GenshinMinecraft/komari-monitor-rs/image?custom_description=Komari+%E7%AC%AC%E4%B8%89%E6%96%B9+Agent+%7C+%E9%AB%98%E6%80%A7%E8%83%BD&description=1&font=KoHo&forks=1&issues=1&language=1&name=1&owner=1&pattern=Floating+Cogs&pulls=1&stargazers=1&theme=Auto)

## 在原版本上的变动
- 更改参数解析库有clap以支持环境变量解析
- 变动文件为src/command_parser.rs，Cargo.toml。
## 近期更新

- 现已支持自动推断 ws_server 参数，若未设置则自动从 http_server 参数中推断
- 现已支持自定义 LogLevel，可用 `--log-level` 参数指定 `error` / `warn` / `info` / `debug` / `trace` (反馈问题请使用 `debug` / `trace`)
- 现已支持更改公网 IP 获取 API，可选 `cloudflare` 与 `ipinfo`
- 现已支持 PTY 功能，可以从管理面板取得 TTY 终端。由于安全问题，需要手动设置 `--terminal` 参数以开启该功能，并可通过 `--terminal-entry` 参数自定义终端入口 (Windows 默认 cmd.exe，其它系统默认 bash)
- exec 后台命令执行功能共用 `--terminal` 参数，若未打开则不会执行主控的命令

## About

`Komari-Monitor-rs` 是一个适用于 [komari-monitor](https://github.com/komari-monitor) 监控服务的第三方**高性能**监控
Agent

致力于实现[原版 Agent](https://github.com/komari-monitor/komari-agent) 的所有功能，并拓展更多功能

## 一键脚本

- 交互模式
  ```bash
  wget -O setup-client-rs.sh "https://ghfast.top/https://raw.githubusercontent.com/GenshinMinecraft/komari-monitor-rs/refs/heads/main/install.sh" && chmod +x setup-client-rs.sh && bash ./setup-client-rs.sh
  ```
- 直接传入参数
  ```bash
  wget -O setup-client-rs.sh "https://ghfast.top/https://raw.githubusercontent.com/GenshinMinecraft/komari-monitor-rs/refs/heads/main/install.sh" && chmod +x setup-client-rs.sh
  bash setup-client-rs.sh --http-server "http://your.server:port" --ws-server "ws://your.server:port" --token "your_token"
  ```

## 与原版的差异

测试项目均在 Redmi Book Pro 15 2022 锐龙版 + Arch Linux 最新版 + Rust Toolchain Stable 下测试

### Binary 体积

原版体积约 6.2M，本项目体积约 992K，相差约 7.1 倍

### 运行内存与 Cpu 占用

原版占用内存约 15.4 MiB，本项目占用内存约 5.53 MB，相差约 2.7 倍

原版峰值 Cpu 占用约 49.6%，本项目峰值 Cpu 占用约 4.8%

并且，本项目在堆上的内存仅 388 kB

### 实现功能

目前，本项目已经实现原版的大部分功能，但还有以下的差异:

- GPU Name 检测

除此之外，还有希望添加的功能:

- 自动更新
- 自动安装
- Bash / PWSH 一键脚本

## 下载

在本项目的 [Release 界面](https://github.com/GenshinMinecraft/komari-monitor-rs/releases/tag/latest) 即可下载，按照架构选择即可

后缀有 `musl` 字样的可以在任何 Linux 系统下运行

后缀有 `gnu` 字样的仅可以在较新的，通用的，带有 `Glibc` 的 Linux 系统下运行，占用会小一些

## Usage

```
Komari Monitor Agent in Rust

Usage: komari-monitor-rs [OPTIONS] --http-server <HTTP_SERVER> --token <TOKEN>

Options:
      --http-server <HTTP_SERVER>
          设置主端 Http 地址
      --ws-server <WS_SERVER>
          设置主端 WebSocket 地址
  -t, --token <TOKEN>
          设置 Token
      --ip-provider <IP_PROVIDER>
          公网 IP 接口 [default: ipinfo] [possible values: cloudflare, ipinfo]
      --terminal
          启用 Terminal (默认关闭)
      --terminal-entry <TERMINAL_ENTRY>
          自定义 Terminal 入口 [default: default]
  -f, --fake <FAKE>
          设置虚假倍率 [default: 1]
      --realtime-info-interval <REALTIME_INFO_INTERVAL>
          设置 Real-Time Info 上传间隔时间 (ms) [default: 1000]
      --tls
          启用 TLS (默认关闭)
      --ignore-unsafe-cert
          忽略证书验证
      --log-level <LOG_LEVEL>
          设置日志等级 (反馈问题请开启 Debug 或者 Trace) [default: info] [possible values: error, warn, info, debug, trace]
  -h, --help
          Print help
  -V, --version
          Print version
```

必须设置 `--http-server` / `--token`

## LICENSE

本项目根据 WTFPL 许可证开源

```
        DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE 
                    Version 2, December 2004 

 Copyright (C) 2004 Sam Hocevar <sam@hocevar.net> 

 Everyone is permitted to copy and distribute verbatim or modified 
 copies of this license document, and changing it is allowed as long 
 as the name is changed. 

            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE 
   TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION 

  0. You just DO WHAT THE FUCK YOU WANT TO.
```
