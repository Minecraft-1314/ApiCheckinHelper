# ApiCheckinHelper-API 签到助手

## 重要公告（必读）

- 安装程序尚未使用代码签名证书，Windows SmartScreen 可能显示安全提示。
- 默认版需要本机安装 .NET Desktop Runtime 8；便携版已内置 .NET 运行时。
- 所有版本都需要 Microsoft Edge WebView2 Runtime。
- 本项目当前只保存和使用 Bearer 访问令牌，不支持账号密码或 Cookie 登录。
- 请勿在问题反馈、日志或截图中粘贴完整访问令牌。

## 项目介绍

这是一个面向 Windows 的本地 API 站点签到与访问令牌管理工具。程序采用 **WPF 外壳 + WebView2 前端 + C# 核心库 + 静态前端资源**的分层结构，支持站点新增与体检、访问令牌管理、结构化批量导入、协议发现、手动签到、批量签到、并发调度、失败重试、取消操作、中英文切换以及深色和浅色主题。

## 下载与安装

| 产物 | 用途 | 运行条件 |
|------|------|------|
| `ApiCheckinHelper-Setup.exe` | Inno Setup 7.1.0 标准安装向导，支持中文和英文 | Windows 10/11 x64，安装时需要管理员权限 |
| `ApiCheckinHelper.msi` | 标准 Windows Installer 安装包 | Windows 10/11 x64，安装时需要管理员权限 |
| `ApiCheckinHelper-default-win-x64.zip` | 默认版压缩包 | 需要 .NET Desktop Runtime 8 和 WebView2 Runtime |
| `ApiCheckinHelper-portable-win-x64.zip` | 单文件便携版压缩包 | 已内置 .NET 运行时，仍需要 WebView2 Runtime |

推荐普通用户使用 `ApiCheckinHelper-Setup.exe`。

## 项目贡献者

| 贡献者 | 贡献内容 |
|------|------|
| Minecraft-1314 | 完整开发 |

欢迎通过 Issues 或 PR 加入贡献者列表。

## 许可协议

本项目采用 MIT 许可证，详情参见 [LICENSE](LICENSE) 文件。

## 支持我们

如果这个项目对您有帮助，欢迎点亮 GitHub 仓库右上角的 Star。
