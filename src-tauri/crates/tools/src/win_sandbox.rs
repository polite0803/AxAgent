// SPDX-License-Identifier: AGPL-3.0-only

//! Windows 受限令牌沙箱（PLAN-codex-parity P0-1b）
//!
//! 对标 codex 的内核级沙箱语义，Windows 侧首阶段实现 `ReadOnly` 模式：
//!
//! ## 原理
//!
//! 1. **SAFER Basic User 令牌**（`runas /trustlevel:0x20000` 同源机制）：
//!    `SaferCreateLevel(SAFER_LEVELID_NORMALUSER)` + `SaferComputeTokenFromLevel`
//!    从当前进程令牌派生 Basic User 令牌——剥离管理员组成员 SID 与全部特权，
//!    并收紧 restricting check：只有「正常检查 + 受限检查」双通过的资源可达。
//!    > 选型说明：手搓 `CreateRestrictedToken`（Chromium USER_LIMITED 配方）
//!    > 在多轮 SID 矩阵实验下均触发 0xC0000142/0xC0000022 子进程初始化失败，
//!    > SAFER 是系统维护的标准配方，实测稳定。
//! 2. `CreateProcessAsUserW` 用 Basic User 令牌启动 `cmd /d /s /c <command>`，
//!    环境块白名单重建（按名称大小写不敏感字母序排序 + 去重——Win32 硬性
//!    要求），命令行缓冲必须显式 null 终止（漏掉会导致分配器复用的堆残留
//!    拼进子进程命令行，产生随机「找不到文件」与输出乱码）。
//! 3. **私有 Window Station + Desktop**：子进程在 `lpDesktop` 指向的私有
//!    桌面运行（DACL 授 Everyone/Users GENERIC_ALL）。默认桌面
//!    `WinSta0\Default` 的 DACL 不含受限令牌的 check SID，conhost 初始化
//!    半途而废，控制台输出出现堆垃圾——私有桌面是必需项而非加固项。
//! 4. 匿名管道收集 stdout/stderr；JobObject（KILL_ON_JOB_CLOSE）兜底进程树清理。
//!
//! ## 当前边界（后续阶段补齐）
//!
//! - 用户自己 Profile 的写入**不受保护**：SAFER NormalUser 是「标准用户」
//!   语义，保留用户对自己目录的权限（实测写入成功）。用户 SID deny-only
//!   层需要 SeAssignPrimaryTokenPrivilege（标准用户不持有）且会被 SAFER
//!   派生重建属性，两条路径均已实测排除；阶段 2 以 AppContainer / 工作区
//!   ACE 方案补齐（codex Windows 沙箱同款思路）。
//! - cwd 必须是 Basic User 令牌可 traverse 的目录（如 `C:\Windows`、盘根）；
//!   用户 Profile 下的目录连读都不行（restricted check 失败）。
//! - `WorkspaceWrite`：需要为受限令牌在工作区目录加 allow ACE
//!   （SetNamedSecurityInfo + 可继承 ACE），见 PLAN-codex-parity P0-1 第二阶段。
//! - 网络封锁：Basic User 令牌不阻断网络；ReadOnly 语义要求禁网，
//!   阶段 2 通过 WFP 或代理出口补齐。当前调用方应将 `network_access=false`
//!   视为「尽力而为」。
//!
//! 非 Windows 平台本模块不编译；Bash 工具侧由 cfg 分支处理。

use std::path::Path;

use axagent_harness::SandboxPolicy;

/// 沙箱化子进程：RAII 兜底——Drop 时 TerminateProcess + Job 句柄关闭
/// （KILL_ON_JOB_CLOSE 终止整个进程树），保证超时/取消不残留进程。
pub struct SandboxedChild {
    process: std::sync::Arc<RawProcess>,
    _job: std::sync::Arc<crate::job_object::JobHandle>,
    /// 管道读取线程的 mpsc 接收端（spawn 时已启动读取线程）
    stdout_rx: Option<std::sync::mpsc::Receiver<Vec<u8>>>,
    stderr_rx: Option<std::sync::mpsc::Receiver<Vec<u8>>>,
}

pub struct SandboxedOutput {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// 进程句柄 RAII 包装（Arc 共享给等待线程）
struct RawProcess(windows_sys::Win32::Foundation::HANDLE);
// SAFETY: HANDLE 是裸句柄；只用于 WaitForSingleObject/TerminateProcess/
// GetExitCodeProcess 等线程安全的 Win32 调用。
unsafe impl Send for RawProcess {}
unsafe impl Sync for RawProcess {}

impl Drop for RawProcess {
    fn drop(&mut self) {
        // SAFETY: self.0 为有效进程句柄（CreateProcessAsUserW 返回），关闭后不再使用。
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

impl Drop for SandboxedChild {
    fn drop(&mut self) {
        self.terminate();
        // _job（Arc<JobHandle>）随 Drop 释放 → 关闭 Job 句柄 → 进程树被终止
    }
}

impl SandboxedChild {
    /// 终止进程树（幂等：已退出的进程 TerminateProcess 失败无副作用）。
    pub fn terminate(&self) {
        // SAFETY: self.process.0 为有效进程句柄。
        unsafe { windows_sys::Win32::System::Threading::TerminateProcess(self.process.0, 1) };
    }

    /// 等待进程退出并收集全部输出（读端 EOF 后 WaitForSingleObject）。
    pub async fn wait_with_output(mut self) -> Result<SandboxedOutput, String> {
        let stdout_rx = self.stdout_rx.take().unwrap_or_else(|| std::sync::mpsc::channel().1);
        let stderr_rx = self.stderr_rx.take().unwrap_or_else(|| std::sync::mpsc::channel().1);
        let process = self.process.clone();
        let (tx, rx) = tokio::sync::oneshot::channel::<SandboxedOutput>();

        // 阻塞等待放在 spawn_blocking，不阻塞 tokio worker
        tokio::task::spawn_blocking(move || {
            // 读端 EOF（子进程/进程树关闭写端）后 recv 返回
            let stdout = stdout_rx.recv().unwrap_or_default();
            let stderr = stderr_rx.recv().unwrap_or_default();
            // SAFETY: process 句柄有效（Arc 保证存活到本闭包结束）。
            unsafe {
                windows_sys::Win32::System::Threading::WaitForSingleObject(
                    process.0,
                    windows_sys::Win32::System::Threading::INFINITE,
                );
            }
            let mut code: u32 = 0;
            // SAFETY: 同上。
            unsafe {
                windows_sys::Win32::System::Threading::GetExitCodeProcess(process.0, &mut code);
            }
            let _ = tx.send(SandboxedOutput { exit_code: code as i32, stdout, stderr });
        });

        rx.await.map_err(|_| "沙箱进程等待任务被取消".to_string())
    }
}

/// 以沙箱策略启动 `cmd /d /s /c <command>`。
///
/// 返回的 [`SandboxedChild`] 具备 RAII 兜底：超时/取消时 Drop 即终止进程树。
pub fn spawn_sandboxed(
    policy: &SandboxPolicy,
    command: &str,
    cwd: &Path,
) -> Result<SandboxedChild, String> {
    match policy.mode {
        axagent_harness::SandboxMode::ReadOnly => spawn_read_only(command, cwd),
        axagent_harness::SandboxMode::WorkspaceWrite => Err(
            "Windows WorkspaceWrite 沙箱尚未实现（需 restricting SID 工作区 ACE，见 PLAN-codex-parity P0-1 第二阶段）"
                .to_string(),
        ),
        axagent_harness::SandboxMode::DangerFullAccess => {
            Err("DangerFullAccess 不应进入沙箱路径（调用方负责走直通分支）".to_string())
        },
    }
}

/// 环境变量白名单：重建环境块，不继承父进程完整 env（防凭据泄露）。
const ENV_WHITELIST: &[&str] = &[
    "SystemRoot",
    "ComSpec",
    "PATHEXT",
    "PATH",
    "OS",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "PROCESSOR_IDENTIFIER",
    "PROCESSOR_LEVEL",
    "PROCESSOR_REVISION",
    "COMPUTERNAME",
    "ProgramData",
    "ProgramFiles",
    "ProgramFiles(x86)",
    "ProgramW6432",
    "CommonProgramFiles",
    "CommonProgramW6432",
    "PUBLIC",
    "windir",
];

/// Reader 端累计上限（字节）：超出后停止读取，防止恶意命令灌爆内存。
/// 命令会因管道写满而阻塞，最终由超时触发 terminate。
const PIPE_READ_CAP_BYTES: u64 = 2 * 1024 * 1024;

fn spawn_read_only(command: &str, cwd: &Path) -> Result<SandboxedChild, String> {
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE_FLAG_INHERIT, SetHandleInformation};
    use windows_sys::Win32::System::Pipes::CreatePipe;
    use windows_sys::Win32::System::Threading::{
        CreateProcessAsUserW, GetCurrentProcess, OpenProcessToken, PROCESS_INFORMATION,
        STARTF_USESTDHANDLES, STARTUPINFOW,
    };

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_UNICODE_ENVIRONMENT: u32 = 0x0000_0400;
    const PIPE_SIZE: u32 = 0;

    // ── 1. 当前进程令牌 ──
    let mut current_token: HANDLE = std::ptr::null_mut();
    // SAFETY: GetCurrentProcess 返回伪句柄；OpenProcessToken 输出指针有效。
    let ok = unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            0x0002 | 0x0008 | 0x0020, // TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY | TOKEN_QUERY
            &mut current_token,
        )
    };
    if ok == 0 {
        return Err("OpenProcessToken 失败".to_string());
    }
    let current_token = HandleGuard(current_token);

    // ── 2. SAFER Basic User 令牌（runas /trustlevel:0x20000 同源机制） ──
    // 剥离管理员组成员 SID 与全部特权、收紧 restricting check —— 「正常检查 +
    // 受限检查」双通过的资源才可达。效果：系统目录/HKLM/Program Files/其他
    // 用户数据一律不可写，世界可读路径照常可读。
    // 已知边界：SAFER NormalUser 是「标准用户」语义，不剥夺用户对自己
    // Profile 的写权限（实测写入成功）。完整 ReadOnly（含用户目录写保护）
    // 需要 SeAssignPrimaryTokenPrivilege 叠加用户 SID deny-only 层——标准
    // 用户令牌不持有该特权（本机实测），留待阶段 2 以 AppContainer /
    // 工作区 ACE 方案实现（codex Windows 沙箱同款思路）。
    // 另注：deny-only 层若置于 SAFER 之前会被 SAFER 派生重建用户 SID 属性
    // 而丢失；置于之后则因 CRT 层破坏 CreateProcessAsUserW 免特权路径而
    // 报 1314——两者均已实测排除。
    use windows_sys::Win32::Security::AppLocker::{
        SAFER_LEVEL_OPEN, SAFER_LEVELID_NORMALUSER, SAFER_SCOPEID_USER, SaferCloseLevel,
        SaferComputeTokenFromLevel, SaferCreateLevel,
    };
    let mut level: windows_sys::Win32::Security::SAFER_LEVEL_HANDLE = std::ptr::null_mut();
    // SAFETY: level 输出指针有效。
    let rc = unsafe {
        SaferCreateLevel(
            SAFER_SCOPEID_USER,
            SAFER_LEVELID_NORMALUSER,
            SAFER_LEVEL_OPEN,
            &mut level,
            std::ptr::null(),
        )
    };
    if rc == 0 {
        return Err(format!("SaferCreateLevel 失败（GetLastError={}）", unsafe {
            windows_sys::Win32::Foundation::GetLastError()
        }));
    }
    let mut spawn_token: HANDLE = std::ptr::null_mut();
    // SAFETY: level 与 current_token 均为有效句柄；spawn_token 输出指针有效。
    // 源令牌需以 TOKEN_DUPLICATE 打开（已带 0x0002）。
    let rc = unsafe {
        SaferComputeTokenFromLevel(
            level,
            current_token.raw(),
            &mut spawn_token,
            0,
            std::ptr::null_mut(),
        )
    };
    // SAFETY: level 为有效句柄，无论成败都不再使用。
    unsafe { SaferCloseLevel(level) };
    if rc == 0 {
        return Err(format!("SaferComputeTokenFromLevel 失败（GetLastError={}）", unsafe {
            windows_sys::Win32::Foundation::GetLastError()
        }));
    }
    let _spawn_token = HandleGuard(spawn_token);

    // ── 3. 匿名管道（stdout / stderr），句柄全部 RAII ──
    let mut out_read: HANDLE = std::ptr::null_mut();
    let mut out_write: HANDLE = std::ptr::null_mut();
    let mut err_read: HANDLE = std::ptr::null_mut();
    let mut err_write: HANDLE = std::ptr::null_mut();
    // SAFETY: 输出指针均有效。
    let ok = unsafe { CreatePipe(&mut out_read, &mut out_write, std::ptr::null(), PIPE_SIZE) };
    if ok == 0 {
        return Err("CreatePipe(stdout) 失败".to_string());
    }
    // SAFETY: 同上。
    let ok = unsafe { CreatePipe(&mut err_read, &mut err_write, std::ptr::null(), PIPE_SIZE) };
    if ok == 0 {
        return Err("CreatePipe(stderr) 失败".to_string());
    }
    let out_read = HandleGuard(out_read);
    let err_read = HandleGuard(err_read);
    let out_write = HandleGuard(out_write);
    let err_write = HandleGuard(err_write);

    // 子进程写端需可继承；父进程读端不可继承（默认即不可继承）
    // SAFETY: 句柄有效。
    let ok =
        unsafe { SetHandleInformation(out_write.raw(), HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) };
    if ok == 0 {
        return Err("SetHandleInformation(stdout) 失败".to_string());
    }
    // SAFETY: 句柄有效。
    let ok =
        unsafe { SetHandleInformation(err_write.raw(), HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) };
    if ok == 0 {
        return Err("SetHandleInformation(stderr) 失败".to_string());
    }

    // ── 4. 环境块（白名单重建，排序 + 去重） ──
    let env_block = build_env_block();

    // ── 5. 命令行：cmd /d /s /c "<command>" ──
    let cmd_path = resolve_cmd_path();
    let cmdline_str = format!("\"{cmd_path}\" /d /s /c \"{command}\"");
    // 注意：必须以 null 终止！否则 Vec 溢出容量上的堆残留（分配器复用的
    // 旧字符串，常见为 env/PATH 碎片）会拼进子进程命令行——cmd /s 模式把
    // 引号外尾随文本并入命令，造成随机的「找不到文件/路径」与输出乱码
    // （曾导致沙箱测试非确定性失败）。
    let mut cmdline: Vec<u16> = cmdline_str.encode_utf16().chain(std::iter::once(0)).collect();
    let cwd_str = cwd.to_string_lossy().to_string();
    let mut cwd_wide: Vec<u16> = cwd_str.encode_utf16().collect();
    cwd_wide.push(0);
    let appname: Vec<u16> = cmd_path.encode_utf16().chain(std::iter::once(0)).collect();

    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdInput = std::ptr::null_mut();
    si.hStdOutput = out_write.raw();
    si.hStdError = err_write.raw();

    // ── 6. 私有 Window Station Desktop ──
    let station = sandbox_station()?;
    let mut desktop_wide: Vec<u16> = station.desktop_path.encode_utf16().chain([0]).collect();
    // lpDesktop 字段类型为 PWSTR（*mut u16），STARTUPINFOW 不会就地修改该缓冲区，
    // 但类型要求可变指针。默认桌面 DACL 不含受限令牌的 check SID，conhost
    // 初始化会半途而废（输出堆垃圾），私有桌面是必需项。
    si.lpDesktop = desktop_wide.as_mut_ptr();

    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    // SAFETY: 所有指针/缓冲区均在调用期间有效；cmdline 为 *mut u16（Win32 可能
    // 就地修改，传出的 Vec 所有权保留在此作用域内）。
    let ok = unsafe {
        CreateProcessAsUserW(
            spawn_token,
            appname.as_ptr(),
            cmdline.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1, // bInheritHandles：继承管道写端
            CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT,
            env_block.as_ptr().cast(),
            cwd_wide.as_ptr(),
            &si,
            &mut pi,
        )
    };
    if ok == 0 {
        return Err(format!("CreateProcessAsUserW 失败（GetLastError={}）", unsafe {
            windows_sys::Win32::Foundation::GetLastError()
        }));
    }
    // SAFETY: hThread 为 CreateProcess 返回的有效线程句柄，父进程不需要。
    unsafe { CloseHandle(pi.hThread) };

    // ── 7. JobObject 兜底进程树清理 ──
    // SAFETY: pi.hProcess 是 CreateProcessAsUserW 刚返回的有效进程句柄。
    let job = match unsafe { crate::job_object::assign_job_raw(pi.hProcess) } {
        Ok(j) => j,
        Err(e) => {
            // 关联失败时手动终止，防进程泄漏
            // SAFETY: pi.hProcess 为有效进程句柄。
            unsafe { windows_sys::Win32::System::Threading::TerminateProcess(pi.hProcess, 1) };
            return Err(format!("关联 JobObject 失败: {e}"));
        },
    };
    let job = std::sync::Arc::new(job);

    // ── 8. 句柄移交 + 输出读取线程 ──
    // 读端转给 reader 线程（File 接管并负责关闭）；写端已被子进程继承，
    // 父进程必须关闭，否则读端永远收不到 EOF。守卫 forget 移交后 Drop 不再重复关闭。
    let out_read_raw = out_read.forget();
    let err_read_raw = err_read.forget();
    let out_write_raw = out_write.forget();
    let err_write_raw = err_write.forget();
    // SAFETY: 有效句柄，一次性关闭（所有权已移交，后续无人再关）。
    unsafe {
        CloseHandle(out_write_raw);
        CloseHandle(err_write_raw);
    }

    let (stdout_tx, stdout_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let (stderr_tx, stderr_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    spawn_pipe_reader(out_read_raw as isize, stdout_tx);
    spawn_pipe_reader(err_read_raw as isize, stderr_tx);

    Ok(SandboxedChild {
        process: std::sync::Arc::new(RawProcess(pi.hProcess)),
        _job: job,
        stdout_rx: Some(stdout_rx),
        stderr_rx: Some(stderr_rx),
    })
}

/// 启动阻塞读取线程：读端读到 EOF（或写端全部关闭）后把累计输出发给 tx。
fn spawn_pipe_reader(read_end: isize, tx: std::sync::mpsc::Sender<Vec<u8>>) {
    use std::io::Read;
    use std::os::windows::io::RawHandle;
    std::thread::spawn(move || {
        // SAFETY: read_end 为有效管道读句柄；File 接管后由其 Drop 关闭。
        // 句柄以 isize 传递（*mut c_void 非Send，无法直接 move 进线程）。
        let file: std::fs::File =
            unsafe { std::os::windows::io::FromRawHandle::from_raw_handle(read_end as RawHandle) };
        let mut buf = Vec::new();
        let _ = file.take(PIPE_READ_CAP_BYTES).read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
}

/// 构建白名单环境块（UTF-16，`K=V\0` 序列 + 终止 `\0`）。
///
/// Win32 硬性要求：环境块字符串必须按变量名**大小写不敏感字母序排序**
/// （CreateProcess 文档明确要求；实测乱序环境块会导致子进程环境表/堆
/// 损坏——cmd 回显追加乱路径碎片、type/dir 随机报「找不到文件/路径」）。
/// 同名变量（大小写变体，Windows 环境可能出现 `Path` 与 `PATH` 并存）只保留首个。
fn build_env_block() -> Vec<u16> {
    let whitelist: Vec<String> = ENV_WHITELIST.iter().map(|s| s.to_ascii_lowercase()).collect();
    let mut entries: Vec<String> = Vec::new();
    for (k, v) in std::env::vars() {
        if whitelist.iter().any(|w| *w == k.to_ascii_lowercase())
            && !entries
                .iter()
                .any(|e: &String| e.split('=').next().is_some_and(|n| n.eq_ignore_ascii_case(&k)))
        {
            entries.push(format!("{k}={v}"));
        }
    }
    // 大小写不敏感字母序（Win32 要求）
    entries.sort_by(|a, b| {
        a.split('=')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .cmp(&b.split('=').next().unwrap_or("").to_ascii_lowercase())
    });
    let mut block: Vec<u16> = Vec::new();
    for e in &entries {
        block.extend(e.encode_utf16());
        block.push(0);
    }
    block.push(0);
    block
}

/// 解析 cmd.exe 绝对路径（COMSPEC 优先，回退系统目录）
fn resolve_cmd_path() -> String {
    std::env::var("ComSpec").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| {
        let sys_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        format!("{sys_root}\\System32\\cmd.exe")
    })
}

/// 私有 Window Station + Desktop（进程级单例）。
///
/// 受限令牌访问 `WinSta0\Default` 桌面时 restricted check 失败（GetLastError=5，
/// 桌面 DACL 只授予用户/SYSTEM/Administrators，不含 Everyone/Users）。
/// Chrome sandbox 同款解法：创建私有 window station + desktop，
/// DACL 授予 Everyone/Everyone-Users `GENERIC_ALL`（SDDL `D:(A;;GA;;;WD)(A;;GA;;;BU)`），
/// 子进程在该桌面上运行（CREATE_NO_WINDOW，无 UI，仅管道 IO）。
///
/// 句柄与 SD 有意泄漏（进程生命周期 = 沙箱生命周期）；station 名由系统按登录
/// 会话 LUID 自动生成（非提权进程不能指定名称），同会话多实例天然复用。
struct SandboxStation {
    /// HWINSTA（isize 存储：裸句柄非 Send，静态缓存需要）
    _hwinsta: isize,
    /// HDESK
    _hdesk: isize,
    /// `"<winsta>\\<desktop>"`，直接填入 STARTUPINFOW.lpDesktop
    desktop_path: String,
}

fn sandbox_station() -> Result<&'static SandboxStation, String> {
    static STATION: std::sync::OnceLock<Result<SandboxStation, String>> =
        std::sync::OnceLock::new();

    STATION.get_or_init(|| {
        use windows_sys::Win32::Foundation::{GENERIC_ALL, HANDLE};
        use windows_sys::Win32::Security::Authorization::
            ConvertStringSecurityDescriptorToSecurityDescriptorW;
        use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
        use windows_sys::Win32::System::StationsAndDesktops::{
            CreateDesktopW, CreateWindowStationW, GetUserObjectInformationW, GetProcessWindowStation,
            SetProcessWindowStation, UOI_NAME,
        };

        const SDDL_REVISION_1: u32 = 1;

        // 1. SDDL → SECURITY_DESCRIPTOR：Everyone + Users 均为 GENERIC_ALL
        let sddl: Vec<u16> = "D:(A;;GA;;;WD)(A;;GA;;;BU)".encode_utf16().chain([0]).collect();
        let mut sd: *mut std::ffi::c_void = std::ptr::null_mut();
        // SAFETY: sddl 以 null 结尾且调用期间有效；输出指针有效。
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err("ConvertStringSecurityDescriptorToSecurityDescriptorW 失败".to_string());
        }
        // SD 有意泄漏：window station/desktop 引用它，进程退出统一回收。
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd,
            bInheritHandle: 0,
        };

        // 2. 私有 window station
        // 注意：MSDN 规定「仅管理员组成员可以指定名称」——未提权进程带名创建
        // 必然 ERROR_ACCESS_DENIED（已实测确认）。因此传 NULL 名称，由系统按
        // 登录会话 LUID 自动命名（"Service-0x0-<luid>$"），随后查询实际站名。
        // 同一登录会话内重复调用会返回已有站点句柄，安全幂等。
        let desktop_name = "sbx";
        // SAFETY: sa 指针有效；NULL 名称表示由系统自动命名。
        let hwinsta = unsafe { CreateWindowStationW(std::ptr::null(), 0, GENERIC_ALL, &sa) };
        if hwinsta.is_null() {
            return Err(format!(
                "CreateWindowStationW 失败（GetLastError={}）",
                unsafe { windows_sys::Win32::Foundation::GetLastError() }
            ));
        }

        // 查询自动生成的站名（UOI_NAME），用于拼接 STARTUPINFOW.lpDesktop
        let mut name_buf = [0u16; 128];
        let mut name_need: u32 = 0;
        // SAFETY: hwinsta 为刚创建的有效句柄；缓冲区与长度匹配。
        let ok = unsafe {
            GetUserObjectInformationW(
                hwinsta,
                UOI_NAME,
                name_buf.as_mut_ptr().cast(),
                (name_buf.len() * 2) as u32,
                &mut name_need,
            )
        };
        if ok == 0 {
            return Err(format!(
                "GetUserObjectInformationW(UOI_NAME) 失败（GetLastError={}）",
                unsafe { windows_sys::Win32::Foundation::GetLastError() }
            ));
        }
        let name_len = name_buf.iter().position(|&c| c == 0).unwrap_or(0);
        let winsta_name = String::from_utf16_lossy(&name_buf[..name_len]);

        // 3. 切换到新 winsta 后建 desktop，随后切回
        // SAFETY: 句柄有效。
        let prev = unsafe { GetProcessWindowStation() };
        // SAFETY: 句柄有效。
        let ok = unsafe { SetProcessWindowStation(hwinsta) };
        if ok == 0 {
            return Err("SetProcessWindowStation(沙箱) 失败".to_string());
        }
        let desktop_wide: Vec<u16> = desktop_name.encode_utf16().chain([0]).collect();
        // SAFETY: 名称与 sa 有效；devmode 为 null。
        let hdesk = unsafe {
            CreateDesktopW(
                desktop_wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                GENERIC_ALL,
                &sa,
            )
        };
        // SAFETY: 句柄有效；无论 desktop 是否建成，都切回原 window station。
        let _ = unsafe { SetProcessWindowStation(prev) };
        if hdesk.is_null() {
            return Err(format!(
                "CreateDesktopW 失败（GetLastError={}）",
                unsafe { windows_sys::Win32::Foundation::GetLastError() }
            ));
        }

        Ok(SandboxStation {
            _hwinsta: hwinsta as HANDLE as isize,
            _hdesk: hdesk as HANDLE as isize,
            desktop_path: format!("{winsta_name}\\{desktop_name}"),
        })
    })
    .as_ref()
    .map_err(|e| e.clone())
}

/// Win32 句柄 RAII 守卫
struct HandleGuard(windows_sys::Win32::Foundation::HANDLE);

impl HandleGuard {
    fn raw(&self) -> windows_sys::Win32::Foundation::HANDLE {
        self.0
    }

    /// 放弃守卫所有权，返回裸句柄（移交子进程 / reader 线程时使用）
    fn forget(self) -> windows_sys::Win32::Foundation::HANDLE {
        let h = self.0;
        std::mem::forget(self);
        h
    }
}

impl Drop for HandleGuard {
    fn drop(&mut self) {
        // SAFETY: self.0 为有效句柄，Drop 后不再使用。
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

// ── 测试（spike 验证，Windows only） ──────────────────────────────────

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use axagent_harness::SandboxPolicy;

    fn read_only_policy() -> SandboxPolicy {
        // 注意：SAFER Basic User 令牌的 restricted check 无法通过仅用户 SID
        // 授权的资源 —— cwd 必须用世界可读目录（如 C:\Windows）。
        // temp_dir() 在用户 Profile 下，子进程初始化打开 cwd 就会失败。
        SandboxPolicy::read_only(std::path::PathBuf::from("C:\\Windows"))
    }

    /// 受限令牌能运行基本命令（restricted check 对 Everyone/Users ACE 放行）
    #[tokio::test]
    async fn sandboxed_echo_works() {
        let policy = read_only_policy();
        let child = spawn_sandboxed(&policy, "echo hello_sandbox", &policy.workspace_cwd)
            .expect("受限令牌 spawn 应成功");
        let output = child.wait_with_output().await.expect("等待输出失败");
        assert_eq!(
            output.exit_code,
            0,
            "echo 应退出码 0，stderr={:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hello_sandbox"), "stdout 应包含回显: {stdout}");
    }

    /// Basic User 令牌不能写系统目录（deny 语义的核心验证）。
    /// 注意：用户自己 Profile 的写入在 v1 不受保护（SAFER NormalUser 保留
    /// 标准用户语义），见模块文档「当前边界」。
    #[tokio::test]
    async fn sandboxed_cannot_write_system_dir() {
        let policy = read_only_policy();
        let cmd = "echo blocked > \"C:\\Windows\\axagent_sandbox_write_probe.txt\"";
        let child =
            spawn_sandboxed(&policy, cmd, &policy.workspace_cwd).expect("受限令牌 spawn 应成功");
        let output = child.wait_with_output().await.expect("等待输出失败");
        assert_ne!(
            output.exit_code,
            0,
            "写入系统目录必须失败（受限令牌 deny 生效），stdout={:?}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            !std::path::Path::new("C:\\Windows\\axagent_sandbox_write_probe.txt").exists(),
            "探测文件不应被创建"
        );
    }

    /// 环境驱动探针（#[ignore]，仅实验诊断用）：AXAGENT_PROBE_CWD /
    /// AXAGENT_PROBE_CMD 控制 cwd 与命令。
    #[tokio::test]
    #[ignore]
    async fn sandbox_env_probe() {
        let cwd = std::env::var("AXAGENT_PROBE_CWD").unwrap_or_else(|_| "C:\\Windows".into());
        let cmd = std::env::var("AXAGENT_PROBE_CMD").unwrap_or_else(|_| "echo hi".into());
        let policy = SandboxPolicy::read_only(std::path::PathBuf::from(&cwd));
        let child = spawn_sandboxed(&policy, &cmd, &policy.workspace_cwd).expect("spawn 应成功");
        let output = child.wait_with_output().await.expect("等待输出失败");
        println!("cmd={cmd:?} cwd={cwd:?}");
        println!("exit={}", output.exit_code);
        println!("stdout={:?}", String::from_utf8_lossy(&output.stdout));
        println!("stderr={:?}", String::from_utf8_lossy(&output.stderr));
    }

    /// Basic User 令牌能读系统文件内容（只读能力保留）
    #[tokio::test]
    async fn sandboxed_can_read_system_dir() {
        let policy = read_only_policy();
        let child = spawn_sandboxed(&policy, "type C:\\Windows\\win.ini", &policy.workspace_cwd)
            .expect("受限令牌 spawn 应成功");
        let output = child.wait_with_output().await.expect("等待输出失败");
        assert_eq!(
            output.exit_code,
            0,
            "type 应成功，stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("[fonts]"),
            "应能读到 win.ini 内容"
        );
    }
}
