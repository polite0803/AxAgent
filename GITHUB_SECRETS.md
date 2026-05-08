# GitHub Actions Secrets 配置指南

## Android 构建所需的 Secrets

### 1. Keystore（用于签名 APK/AAB）

#### 选项 A：创建新的 Keystore（推荐用于新发布）

```bash
keytool -genkeypair -v \
  -keystore axagent-release.jks \
  -keyalg RSA \
  -keysize 2048 \
  -validity 10000 \
  -alias axagent \
  -storepass YOUR_STORE_PASSWORD \
  -keypass YOUR_KEY_PASSWORD \
  -dname "CN=AxAgent, OU=Development, O=AxAgent, L=Beijing, S=Beijing, C=CN"
```

然后将 keystore 转换为 base64：

```bash
base64 axagent-release.jks > keystore.b64
```

#### 选项 B：在 GitHub 中添加 Secrets

| Secret 名称 | 值 | 说明 |
|------------|-----|------|
| `KEYSTORE_FILE` | base64 编码的 keystore 内容 | 运行上面的命令获取 |
| `KEYSTORE_PASSWORD` | 你的 keystore 密码 | 创建 keystore 时设置的密码 |
| `KEY_ALIAS` | `axagent` | 创建 keystore 时使用的别名 |
| `KEY_PASSWORD` | 你的 key 密码 | 创建 keystore 时设置的密码 |

### 2. Tauri Updater 签名（已有）

| Secret 名称 | 值 | 说明 |
|------------|-----|------|
| `TAURI_SIGNING_PRIVATE_KEY` | 已有的 Tauri 私钥 | 用于更新验证 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码 | 已有的 |

## iOS 构建所需的 Secrets（可选）

iOS 签名需要 Apple Developer 账号和证书。如果不配置这些 secrets，工作流仍会构建但不签名（用于测试/模拟器）。

| Secret 名称 | 值 | 说明 |
|------------|-----|------|
| `APPLE_CERTIFICATE` | base64 编码的 .p12 证书 | 从 Xcode 导出 |
| `APPLE_CERTIFICATE_PASSWORD` | 证书密码 | 导出时设置的密码 |
| `APPLE_ID` | Apple Developer 账号邮箱 | 如 `dev@example.com` |
| `APPLE_PASSWORD` | App-Specific Password | 在 appleid.apple.com 生成 |
| `APPLE_TEAM_ID` | Apple Team ID | 如 `AB12CDE3FG` |

### 导出 Apple 证书的步骤

1. 打开 Keychain Access（macOS）
2. 找到你的 Apple Development/Distribution 证书
3. 右键 → Export "Apple Distribution: ..."
4. 保存为 `.p12` 文件，设置密码
5. 转换为 base64：

```bash
base64 AxAgent_Distribution.p12 > apple_cert.b64
```

## 如何添加 GitHub Secrets

1. 打开你的 GitHub 仓库
2. 进入 **Settings** → **Secrets and variables** → **Actions**
3. 点击 **New repository secret**
4. 添加上述所有需要的 secrets

## 验证 Secrets 是否配置正确

推送一个 tag 后，检查 Actions 日志：
- 如果看到 "Setting up keystore" 且没有错误，说明 Android 签名配置正确
- 如果看到 "Building without signing" 警告，说明签名 secrets 未配置（不影响测试构建）
