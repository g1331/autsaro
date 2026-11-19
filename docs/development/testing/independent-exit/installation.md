# B4 安装包实测

由 [#63](https://github.com/g1331/autsaro/issues/63) 承接。按[发布说明](../../../maintainers/releasing.md)取得同一冻结提交的
Linux／Windows 安装介质，保存工件来源和 SHA-256。每个平台使用断网、无源码的干净 VM。Linux 预装目标包要求的依赖，
Windows 从管理员 PowerShell 7 操作。先安装，通过工作台手工检查后再卸载。

## 安装

Linux 选择同一 SHA 的 `.deb`。其余约定的交付形式按发行说明另验。在结果目录运行：

```bash
DEB=/absolute/path/to/bundle.deb
sha256sum "$DEB" > install-medium.sha256
set -o pipefail
sudo apt-get --no-download install "$DEB" 2>&1 | tee install.log
install_code=${PIPESTATUS[0]}
printf '%s\n' "$install_code" > install-exit.txt
if [ "$install_code" -eq 0 ]; then
  PACKAGE=$(dpkg-deb -f "$DEB" Package)
  dpkg-query --show "$PACKAGE"
  dpkg-query -L "$PACKAGE" | grep '/bin/'
fi
```

Windows 将所选 `.msi` 放入结果目录：

```powershell
$Msi = (Resolve-Path './bundle.msi').Path
Get-FileHash $Msi | Out-File install-medium.txt
$Install = Start-Process msiexec.exe -Wait -PassThru -ArgumentList "/i `"$Msi`" /qn /norestart /L*v install.log"
$Install.ExitCode | Set-Content install-exit.txt
```

在退出码为 `0` 且无重启要求时进入手工检查。`3010` 表示安装成功但须重启才能完成，记下状态并手工重启 VM。
收到 `1641` 时等待已启动的重启完成。日志或系统明确要求重启时也先重启，再继续手工检查。其余非零码判安装失败，
保留介质、退出码和原始日志。[Windows Installer 返回码](https://learn.microsoft.com/en-us/windows/win32/msi/error-codes)

## 手工检查

1. 从安装后的程序路径或开始菜单启动，确认工作台打开且可操作。
2. 从移交的 ARXML 创建工程并保存。退出应用后再次启动并打开工程，确认原输入和值一致。
3. 通过工作台实际导出一次交付包，确认目标目录包含清单及生成文件。记录安装版路径、截图、工程与导出目录，以及权限或命名空间限制。

以上三项均完成并保留证据后，再执行卸载。

## 卸载

Linux 从同一结果目录执行。重新指定原安装介质绝对路径：

```bash
DEB=/absolute/path/to/bundle.deb
PACKAGE=$(dpkg-deb -f "$DEB" Package)
set -o pipefail
sudo apt-get --no-download remove "$PACKAGE" 2>&1 | tee uninstall.log
uninstall_code=${PIPESTATUS[0]}
printf '%s\n' "$uninstall_code" > uninstall-exit.txt
```

Windows 在原结果目录执行，重启后重新选定同一介质：

```powershell
$Msi = (Resolve-Path './bundle.msi').Path
$Remove = Start-Process msiexec.exe -Wait -PassThru -ArgumentList "/x `"$Msi`" /qn /norestart /L*v uninstall.log"
$Remove.ExitCode | Set-Content uninstall-exit.txt
```

确认卸载退出码及日志。Windows 出现 `3010`／`1641` 时，重启完成后再核对应用登记和安装路径已移除。
