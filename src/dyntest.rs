//! 动态编译运行 harness（仅开发期使用）：将代码片段生成临时 Cargo 工程并编译执行，支持超时、批量与缓存（`DnyRun` / `BatchRunner`）。
//!
//! 需要同时启用 `"dyntest"` 与 `"std"` 特性。传入代码将被编译执行，**仅可在隔离沙箱中使用**。
#![allow(unused)]
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::rand;

/// 动态测试运行的系统级错误（区别于被测程序自身的失败）。
///
/// 未来可能新增变体，请勿依赖穷尽匹配（匹配时请保留通配分支）。
///
/// # Feature Requirement
///
/// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SystemError {
    /// 无系统级错误；`ok == false` 时表示被测程序自身返回非零退出码。
    None,
    /// 执行超时并已被终止，载荷为超时阈值。
    Timeout(Duration),
    /// `cargo build` 失败（退出码非零）。
    CompileFailed,
    /// [`DnyRun::cargo`](crate::dyntest::DnyRun::cargo) 执行的自定义 `cargo <args>`
    /// 命令返回非零退出码（超时、启动失败除外，分别记为 `Timeout` / `ProcessSpawnFailed`）。
    CargoFailed,
    /// 子进程无法启动，载荷为系统错误描述。
    ProcessSpawnFailed(String),
}

/// 单次动态测试的结果，`Display` 输出彩色摘要（超长输出自动截断）。
///
/// # Feature Requirement
///
/// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::dyntest::{BatchRunner, DnyTask};
///
/// let task = DnyTask::new("hello", r#"fn main() { println!("hi"); }"#);
/// let results = BatchRunner::new().run([task]);
/// assert_eq!(results.len(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct DnyResult {
    /// 被测程序标准输出全文。
    pub stdout: String,
    /// 被测程序标准错误全文（已过滤 `Compiling` / `Finished` 行）。
    pub stderr: String,
    /// 被测程序退出码；系统级失败时为 `-2`，默认构造时为 `-1`。
    pub exit_code: i32,
    /// 编译耗时。
    pub build_duration: Duration,
    /// 运行耗时。
    pub run_duration: Duration,
    /// 被测程序是否以退出码 0 正常结束。
    pub ok: bool,
    /// 系统级错误，无则为 [`SystemError::None`]。
    pub system_err: SystemError,
}

impl Default for DnyResult {
    fn default() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: -1,
            build_duration: Duration::ZERO,
            run_duration: Duration::ZERO,
            ok: false,
            system_err: SystemError::None,
        }
    }
}

fn truncate_output(text: &str, max_lines: usize) -> String {
    const MAX_LINE_CHARS: usize = 300;

    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }

    let truncate_line = |line: &str| -> String {
        if line.len() <= MAX_LINE_CHARS {
            return line.to_string();
        }

        let mut chars = line.chars();
        let head: String = chars.by_ref().take(MAX_LINE_CHARS / 2).collect();

        if chars.next().is_none() {
            return head;
        }

        let tail: String = line
            .chars()
            .rev()
            .take(MAX_LINE_CHARS / 2)
            .collect::<Vec<char>>()
            .into_iter()
            .rev()
            .collect();

        format!("{} ... [单行超长截断] ... {}", head, tail)
    };

    let lines: Vec<&str> = text.lines().collect();

    if lines.len() <= max_lines {
        return lines
            .into_iter()
            .map(truncate_line)
            .collect::<Vec<_>>()
            .join("\n");
    }

    let half = max_lines / 2;
    let head = lines[..half]
        .iter()
        .copied()
        .map(truncate_line)
        .collect::<Vec<_>>()
        .join("\n");
    let tail = lines[lines.len() - half..]
        .iter()
        .copied()
        .map(truncate_line)
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "{}\n\n... [已截断 {} 行] ...\n\n{}",
        head,
        lines.len() - max_lines,
        tail
    )
}

impl fmt::Display for DnyResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let green = "\x1b[32m";
        let red = "\x1b[31m";
        let yellow = "\x1b[33m";
        let cyan = "\x1b[36m";
        let reset = "\x1b[0m";

        let (status_color, status_text) = match &self.system_err {
            SystemError::None if self.ok => (green, "SUCCESS"),
            SystemError::None => (red, "FAILED (Runtime Error)"),
            SystemError::Timeout(_) => (yellow, "TIMEOUT (TLE)"),
            SystemError::CompileFailed => (red, "COMPILE ERROR (CE)"),
            SystemError::CargoFailed => (red, "CARGO ERROR (CE)"),
            SystemError::ProcessSpawnFailed(_) => (red, "SYSTEM ERROR"),
        };

        writeln!(f, "{status_color}=== [Result: {}] ==={reset}", status_text)?;
        writeln!(
            f,
            "> Build: {cyan}{:?}{reset} | Run: {cyan}{:?}{reset}",
            self.build_duration, self.run_duration
        )?;
        writeln!(f, "> Exit Code: {status_color}{}{reset}", self.exit_code)?;

        let stdout_trimmed = self.stdout.trim();
        if !stdout_trimmed.is_empty() {
            writeln!(
                f,
                "{}--- Stdout (size: {} bytes) ---{}",
                yellow,
                self.stdout.len(),
                reset
            )?;
            writeln!(f, "{}", truncate_output(stdout_trimmed, 20))?;
        }

        let clean_stderr = self
            .stderr
            .lines()
            .filter(|l| !l.trim().starts_with("Compiling "))
            .filter(|l| !l.trim().starts_with("Finished "))
            .collect::<Vec<_>>()
            .join("\n");

        let stderr_trimmed = clean_stderr.trim();
        if !stderr_trimmed.is_empty() {
            writeln!(
                f,
                "{}--- Stderr (size: {} bytes) ---{}",
                red,
                clean_stderr.len(),
                reset
            )?;
            writeln!(f, "{}", truncate_output(stderr_trimmed, 40))?;
        }

        writeln!(f, "{status_color}=========================={reset}")?;
        Ok(())
    }
}

/// 单个动态测试任务：`main.rs` 内容、依赖与运行配置的 Builder。
///
/// # Feature Requirement
///
/// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
///
/// # Examples
///
/// ```rust
/// use lib_unknown::dyntest::DnyTask;
///
/// let task = DnyTask::new("hello", r#"fn main() { println!("hi"); }"#)
///     .with_deps(r#"serde = "1""#)
///     .with_env("RUST_BACKTRACE", "1")
///     .with_release(false);
/// assert_eq!(task.tag, "hello");
/// ```
#[derive(Clone)]
pub struct DnyTask {
    /// 任务标识，用于结果回调用例区分；`From` 元组缺省时为 `"unnamed_task"`。
    pub tag: String,
    /// 待编译运行的 `main.rs` 源码。
    pub main_code: String,
    /// 追加到临时工程 `[dependencies]` 的依赖声明片段。
    pub deps: String,
    /// 注入子进程的环境变量。
    pub envs: HashMap<String, String>,
    /// 写入临时工程 `.cargo/config.toml` 的配置，无则为 `None`。
    pub cargo_config: Option<String>,
    /// 是否以 `--release` 编译运行。
    pub is_release: bool,
}

impl DnyTask {
    /// 以标识与 `main.rs` 源码创建任务，其余配置取默认值。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_unknown::dyntest::DnyTask;
    ///
    /// let task = DnyTask::new("t", "fn main() {}");
    /// assert_eq!(task.tag, "t");
    /// ```
    pub fn new(tag: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            main_code: code.into(),
            deps: String::new(),
            envs: HashMap::new(),
            cargo_config: None,
            is_release: false,
        }
    }
    /// 设置 `[dependencies]` 依赖声明片段。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn with_deps(mut self, deps: impl Into<String>) -> Self {
        self.deps = deps.into();
        self
    }
    /// 注入单个环境变量。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.insert(key.into(), value.into());
        self
    }
    /// 批量注入环境变量。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn with_envs<I, K, V>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in envs {
            self.envs.insert(k.into(), v.into());
        }
        self
    }
    /// 设置写入临时工程 `.cargo/config.toml` 的配置。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn with_cargo_config(mut self, config: impl Into<String>) -> Self {
        self.cargo_config = Some(config.into());
        self
    }
    /// 开启或关闭 release 模式。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn with_release(mut self, release: bool) -> Self {
        self.is_release = release;
        self
    }
}

impl From<(String, String, Option<String>)> for DnyTask {
    fn from(tuple: (String, String, Option<String>)) -> Self {
        Self {
            main_code: tuple.0,
            deps: tuple.1,
            tag: tuple.2.unwrap_or_else(|| "unnamed_task".to_string()),
            envs: HashMap::new(),
            cargo_config: None,
            is_release: false, // [新增]
        }
    }
}

/// 批量运行器：串行执行多个 [`DnyTask`]，可选超时、仅编译、依赖缓存与逐任务回调。
///
/// # Feature Requirement
///
/// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::dyntest::{BatchRunner, DnyTask};
///
/// let tasks = [DnyTask::new("a", "fn main() {}"), DnyTask::new("b", "fn main() {}")];
/// let results = BatchRunner::new().only_build(true).run(tasks);
/// assert_eq!(results.len(), 2);
/// ```
pub struct BatchRunner {
    timeout: Option<Duration>,
    only_build: bool,
    use_cache: bool,
    on_result: Option<Box<dyn FnMut(String, DnyResult)>>,
}

impl Default for BatchRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchRunner {
    /// 以默认配置（无超时、编译并运行、不缓存、无回调）创建运行器。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn new() -> Self {
        Self {
            timeout: None,
            only_build: false,
            use_cache: false,
            on_result: None,
        }
    }

    /// 设置单个任务编译 + 运行的总超时，超时后子进程被终止，结果记为 [`SystemError::Timeout`]。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout = Some(d);
        self
    }
    /// 仅编译不运行。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn only_build(mut self, b: bool) -> Self {
        self.only_build = b;
        self
    }
    /// 启用首任务依赖预热缓存（先空跑一次编译，后续任务复用编译缓存）。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn use_cache(mut self, c: bool) -> Self {
        self.use_cache = c;
        self
    }
    /// 设置每个任务完成后的回调（参数为任务标识与结果）。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn on_result<F>(mut self, f: F) -> Self
    where
        F: FnMut(String, DnyResult) + 'static,
    {
        self.on_result = Some(Box::new(f));
        self
    }

    pub(crate) fn with_boxed_callback(
        mut self,
        cb: Option<Box<dyn FnMut(String, DnyResult)>>,
    ) -> Self {
        self.on_result = cb;
        self
    }

    /// 串行执行全部任务，返回按输入顺序的 `(标识, 结果)` 列表；空输入直接返回空列表。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lib_unknown::dyntest::{BatchRunner, DnyTask};
    ///
    /// let results = BatchRunner::new().run([DnyTask::new("a", "fn main() {}")]);
    /// assert!(results[0].1.ok);
    /// ```
    ///
    /// # Panics
    ///
    /// - 任务工程目录创建/文件写入失败（磁盘不可写、权限不足）时 panic，源于内部 `DnyRun` 初始化。
    pub fn run<I, T>(mut self, tasks: I) -> Vec<(String, DnyResult)>
    where
        I: IntoIterator<Item = T>,
        T: Into<DnyTask>,
    {
        let tasks: Vec<DnyTask> = tasks.into_iter().map(|t| t.into()).collect();

        if tasks.is_empty() {
            return vec![];
        }

        if self.use_cache {
            self.run_with_cache(tasks)
        } else {
            self.run_standard(tasks)
        }
    }

    fn run_standard(&mut self, tasks: Vec<DnyTask>) -> Vec<(String, DnyResult)> {
        let mut results = Vec::with_capacity(tasks.len());
        for (index, task) in tasks.into_iter().enumerate() {
            let tag = if task.tag == "unnamed_task" {
                format!("task_{}", index)
            } else {
                task.tag
            };

            let mut runner = DnyRun::new(&task.main_code, &task.deps);
            runner.envs(task.envs.clone());
            runner.release(task.is_release);

            if let Some(config) = task.cargo_config {
                runner.cargo_config(config);
            }

            let res = if self.only_build {
                runner.build(self.timeout)
            } else {
                runner.run(self.timeout)
            };

            if let Some(ref mut callback) = self.on_result {
                callback(tag.clone(), res.clone());
            }
            results.push((tag, res));
        }
        results
    }

    fn run_with_cache(&mut self, tasks: Vec<DnyTask>) -> Vec<(String, DnyResult)> {
        let mut results = Vec::with_capacity(tasks.len());
        let first_deps = &tasks[0].deps;

        let mut runner = DnyRun::new("fn main() {}", first_deps);
        runner.release(tasks[0].is_release);

        if let Some(config) = &tasks[0].cargo_config {
            runner.cargo_config(config.clone());
        }

        let init_res = runner.build(self.timeout);
        if let Some(ref mut callback) = self.on_result {
            callback("init_cache".to_string(), init_res.clone());
        }
        results.push(("init_cache".to_string(), init_res));

        for (index, task) in tasks.into_iter().enumerate() {
            let tag = if task.tag == "unnamed_task" {
                format!("task_{}", index)
            } else {
                task.tag
            };

            runner.reset_deps(&task.deps);
            runner.reset_main_code(&task.main_code);

            runner.clear_envs();
            runner.envs(task.envs.clone());
            runner.release(task.is_release); // [新增]

            if let Some(config) = task.cargo_config {
                runner.cargo_config(config);
            } else {
                runner.clear_cargo_config();
            }

            let res = if self.only_build {
                runner.build(self.timeout)
            } else {
                runner.run(self.timeout)
            };

            if let Some(ref mut callback) = self.on_result {
                callback(tag.clone(), res.clone());
            }
            results.push((tag, res));
        }
        results
    }
}

/// 运行单个动态测试任务的便捷入口（等价于单任务 `BatchRunner`）。
///
/// # Feature Requirement
///
/// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::dyntest::dny_run;
///
/// let res = dny_run("fn main() {}", "", None, true);
/// assert!(res.ok);
/// ```
///
/// # Panics
///
/// - 任务工程目录创建/文件写入失败时 panic，源于内部 `DnyRun` 初始化。
pub fn dny_run(
    main_code: &str,
    deps: &str,
    timeout: Option<Duration>,
    only_build: bool,
) -> DnyResult {
    let dny_runner = DnyRun::new(main_code, deps);
    if only_build {
        dny_runner.build(timeout)
    } else {
        dny_runner.run(timeout)
    }
}

/// 批量运行的便捷入口（不使用依赖预热缓存）。
///
/// # Feature Requirement
///
/// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::dyntest::{DnyTask, dny_run_batch};
///
/// let results = dny_run_batch([DnyTask::new("a", "fn main() {}")], None, None, None);
/// assert_eq!(results.len(), 1);
/// ```
///
/// # Panics
///
/// - 任务工程目录创建/文件写入失败时 panic，源于内部 `DnyRun` 初始化。
pub fn dny_run_batch<I, T>(
    task: I,
    task_timeout: Option<Duration>,
    only_build: Option<bool>,
    on_result: Option<Box<dyn FnMut(String, DnyResult)>>,
) -> Vec<(String, DnyResult)>
where
    I: IntoIterator<Item = T>,
    T: Into<DnyTask>,
{
    let mut runner = BatchRunner::new()
        .only_build(only_build.unwrap_or(false))
        .use_cache(false)
        .with_boxed_callback(on_result);

    if let Some(t) = task_timeout {
        runner = runner.timeout(t);
    }
    runner.run(task)
}

/// 批量运行的便捷入口（启用首任务依赖预热缓存，首个结果为 `"init_cache"`）。
///
/// # Feature Requirement
///
/// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::dyntest::{DnyTask, dny_run_batch_use_cache};
///
/// let results = dny_run_batch_use_cache([DnyTask::new("a", "fn main() {}")], None, None, None);
/// assert_eq!(results[0].0, "init_cache");
/// ```
///
/// # Panics
///
/// - 任务工程目录创建/文件写入失败时 panic，源于内部 `DnyRun` 初始化。
pub fn dny_run_batch_use_cache<I, T>(
    task: I,
    task_timeout: Option<Duration>,
    only_build: Option<bool>,
    on_result: Option<Box<dyn FnMut(String, DnyResult)>>,
) -> Vec<(String, DnyResult)>
where
    I: IntoIterator<Item = T>,
    T: Into<DnyTask>,
{
    let mut runner = BatchRunner::new()
        .only_build(only_build.unwrap_or(false))
        .use_cache(true)
        .with_boxed_callback(on_result);

    if let Some(t) = task_timeout {
        runner = runner.timeout(t);
    }
    runner.run(task)
}

/// 单个临时 Cargo 工程的句柄：在 `target/dyn_tests/dny_<id>/` 下生成工程并编译执行，`Drop` 时释放心跳锁。
///
/// # Feature Requirement
///
/// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
pub struct DnyRun {
    /// 待编译运行的 `main.rs` 源码。
    pub main_code: String,
    /// `[dependencies]` 依赖声明片段。
    pub deps: String,
    dir: PathBuf,
    project_name: String,
    lock: DynTestLock,
    envs: HashMap<String, String>,
    cargo_config: Option<String>,
    /// 是否以 `--release` 编译运行。
    pub is_release: bool,
}

impl DnyRun {
    /// 在 `target/dyn_tests` 下创建以纳秒时间戳 + 随机数命名的临时工程并同步文件。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lib_unknown::dyntest::DnyRun;
    ///
    /// let runner = DnyRun::new("fn main() {}", "");
    /// let res = runner.build(None);
    /// assert!(res.ok);
    /// ```
    ///
    /// # Panics
    ///
    /// - 系统时钟早于 UNIX 纪元、无法获取当前目录或工程目录/文件创建失败时 panic。
    pub fn new(main_code: &str, deps: &str) -> Self {
        let id = format!(
            "{}_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            rand::random::<u32>()
        );
        let project_name = format!("dyn_test_{}", id);

        let mut dir = std::env::current_dir().unwrap();
        dir.push("target");
        dir.push("dyn_tests");
        dir.push(format!("dny_{}", id));

        let src_dir = dir.join("src");
        fs::create_dir_all(&src_dir).expect("无法创建共享测试目录");

        let instance = Self {
            main_code: main_code.to_string(),
            deps: deps.to_string(),
            dir,
            project_name: project_name.clone(),
            lock: DynTestLock::new(&project_name),
            envs: HashMap::new(),
            cargo_config: None,
            is_release: false,
        };
        instance.sync_files();
        instance
    }

    /// 设置是否以 `--release` 编译运行。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn release(&mut self, release: bool) -> &mut Self {
        self.is_release = release;
        self
    }

    /// 注入单个子进程环境变量。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn env(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.envs.insert(key.into(), value.into());
        self
    }

    /// 批量注入子进程环境变量。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn envs<I, K, V>(&mut self, envs: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in envs {
            self.envs.insert(k.into(), v.into());
        }
        self
    }

    /// 清空已注入的环境变量。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn clear_envs(&mut self) -> &mut Self {
        self.envs.clear();
        self
    }

    /// 设置 `.cargo/config.toml` 内容并立即同步到工程目录。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Panics
    ///
    /// - `.cargo` 目录或配置文件写入失败时 panic。
    pub fn cargo_config(&mut self, config: impl Into<String>) -> &mut Self {
        self.cargo_config = Some(config.into());
        self.sync_cargo_config();
        self
    }

    /// 清除 cargo 配置（内存值置空并删除已写入的配置文件，忽略删除失败）。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    pub fn clear_cargo_config(&mut self) -> &mut Self {
        self.cargo_config = None;
        let config_path = self.dir.join(".cargo").join("config.toml");
        if config_path.exists() {
            let _ = fs::remove_file(config_path);
        }
        self
    }

    fn sync_cargo_config(&self) {
        if let Some(config_content) = &self.cargo_config {
            let cargo_dir = self.dir.join(".cargo");
            if !cargo_dir.exists() {
                fs::create_dir_all(&cargo_dir).expect("无法创建 .cargo 目录");
            }
            fs::write(cargo_dir.join("config.toml"), config_content)
                .expect("写入 .cargo/config.toml 失败");
        }
    }

    /// 将当前 `Cargo.toml` / `main.rs` / cargo 配置同步到工程目录。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Panics
    ///
    /// - 文件写入失败（磁盘不可写、权限不足）时 panic。
    pub fn sync_files(&self) {
        let cargo_toml = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}
"#,
            self.project_name, self.deps
        );
        fs::write(self.dir.join("Cargo.toml"), cargo_toml).expect("写入 Cargo.toml 失败");
        fs::write(self.dir.join("src").join("main.rs"), &self.main_code)
            .expect("写入 main.rs 失败");

        self.sync_cargo_config();
    }

    fn execute_cmd(
        cmd: &mut Command,
        timeout: Option<Duration>,
    ) -> Result<(Output, Duration), SystemError> {
        let start_time = Instant::now();

        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SystemError::ProcessSpawnFailed(e.to_string()))?;

        let mut stdout_pipe = child.stdout.take().expect("无法获取 stdout");
        let mut stderr_pipe = child.stderr.take().expect("无法获取 stderr");

        let stdout_handle = thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stdout_pipe.read_to_end(&mut buf);
            buf
        });
        let stderr_handle = thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stderr_pipe.read_to_end(&mut buf);
            buf
        });

        match timeout {
            None => {
                let status = child
                    .wait()
                    .map_err(|e| SystemError::ProcessSpawnFailed(e.to_string()))?;
                let stdout = stdout_handle.join().unwrap_or_default();
                let stderr = stderr_handle.join().unwrap_or_default();
                Ok((
                    Output {
                        status,
                        stdout,
                        stderr,
                    },
                    start_time.elapsed(),
                ))
            }
            Some(limit) => {
                let check_interval = Duration::from_millis(10);
                loop {
                    if let Ok(Some(status)) = child.try_wait() {
                        let stdout = stdout_handle.join().unwrap_or_default();
                        let stderr = stderr_handle.join().unwrap_or_default();
                        return Ok((
                            Output {
                                status,
                                stdout,
                                stderr,
                            },
                            start_time.elapsed(),
                        ));
                    }

                    if start_time.elapsed() >= limit {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(SystemError::Timeout(limit));
                    }
                    thread::sleep(check_interval);
                }
            }
        }
    }

    /// 执行 `cargo build`（`is_release` 为真时加 `--release`），超时则终止并记为 [`SystemError::Timeout`]；自身不 panic，失败均体现在返回的 [`DnyResult`] 中。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lib_unknown::dyntest::DnyRun;
    ///
    /// let res = DnyRun::new("fn main() {}", "").build(None);
    /// assert!(res.ok);
    /// ```
    pub fn build(&self, timeout: Option<Duration>) -> DnyResult {
        let mut result = DnyResult::default();
        let mut cmd = Command::new("cargo");
        cmd.arg("build").arg("-q").current_dir(&self.dir);

        if self.is_release {
            cmd.arg("--release");
        }

        cmd.envs(&self.envs);

        match Self::execute_cmd(&mut cmd, timeout) {
            Ok((output, duration)) => {
                result.build_duration = duration;
                result.stdout = String::from_utf8_lossy(&output.stdout).to_string();
                result.stderr = String::from_utf8_lossy(&output.stderr).to_string();
                result.exit_code = output.status.code().unwrap_or(-1);
                result.ok = output.status.success();
                if !result.ok {
                    result.system_err = SystemError::CompileFailed;
                }
            }
            Err(sys_err) => {
                result.system_err = sys_err;
                result.exit_code = -2;
            }
        }
        result
    }

    /// 先编译后直接运行产物；编译失败时直接返回编译结果，不启动进程。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lib_unknown::dyntest::DnyRun;
    ///
    /// let res = DnyRun::new(r#"fn main() { println!("hi"); }"#, "").run(None);
    /// assert!(res.ok);
    /// ```
    pub fn run(&self, timeout: Option<Duration>) -> DnyResult {
        let build_result = self.build(timeout);
        if !build_result.ok {
            return build_result;
        }
        self.run_no_build(timeout, Some(build_result))
    }

    /// 计算当前配置下产物可执行文件的预期路径（不保证文件已存在）。
    ///
    /// 路径由以下因素决定：
    /// - 工程目录下的 `target` 子目录：若 `.cargo/config.toml` 中配置了
    ///   `[build]` 段下以 `target` 开头的键（如 `target` 或 `target-dir`），
    ///   则按配置值拼接实际输出目录；
    /// - [`is_release`](Self::is_release) 决定使用 `release` 还是 `debug` profile 子目录；
    /// - 当前运行平台决定可执行文件后缀（Windows 为 `.exe`，否则无后缀）。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lib_unknown::dyntest::DnyRun;
    ///
    /// let runner = DnyRun::new("fn main() {}", "");
    /// let build_res = runner.build(None);
    /// assert!(build_res.ok);
    ///
    /// let path = runner.bin_path();
    /// assert!(path.exists(), "产物路径应在编译成功后存在: {}", path.display());
    /// ```
    pub fn bin_path(&self) -> PathBuf {
        let mut target_dir = self.dir.join("target");

        if let Some(cfg_str) = &self.cargo_config {
            let mut in_build_section = false;

            for line in cfg_str.lines() {
                let line = line.trim();

                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if line.starts_with('[') {
                    in_build_section = line == "[build]";
                    continue;
                }

                if in_build_section
                    && line.starts_with("target")
                    && let Some((_, val)) = line.split_once('=')
                {
                    let val_no_comment = val.split('#').next().unwrap_or(val);
                    let target_val = val_no_comment
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'');
                    target_dir = target_dir.join(target_val);
                    break;
                }
            }
        }

        let exe_name = if cfg!(windows) {
            format!("{}.exe", self.project_name)
        } else {
            self.project_name.clone()
        };

        let profile_dir = if self.is_release { "release" } else { "debug" };
        target_dir.join(profile_dir).join(exe_name)
    }

    /// 不重新编译，直接运行已有产物（产物缺失时返回 [`SystemError::ProcessSpawnFailed`] 结果）。
    /// `build_result` 用于透传编译耗时，无则置零。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lib_unknown::dyntest::DnyRun;
    ///
    /// let runner = DnyRun::new("fn main() {}", "");
    /// let build_res = runner.build(None);
    /// assert!(build_res.ok);
    ///
    /// let res = runner.run_no_build(None, None);
    /// assert!(res.ok);
    /// ```
    pub fn run_no_build(
        &self,
        timeout: Option<Duration>,
        build_result: Option<DnyResult>,
    ) -> DnyResult {
        let bin_path = self.bin_path();

        let build_result = build_result.unwrap_or_default();
        let mut result = DnyResult {
            build_duration: build_result.build_duration,
            ..Default::default()
        };

        if !bin_path.exists() {
            result.system_err = SystemError::ProcessSpawnFailed(format!(
                "测试可执行文件未找到。\n预期路径: {}\n请检查编译是否真的成功，或者 target/profile 逻辑是否正确。",
                bin_path.display()
            ));
            result.exit_code = -2;
            return result;
        }

        let mut cmd = Command::new(&bin_path);
        cmd.envs(&self.envs);

        match Self::execute_cmd(&mut cmd, timeout) {
            Ok((output, duration)) => {
                result.run_duration = duration;
                result.stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                result.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                result.exit_code = output.status.code().unwrap_or(-1);
                result.ok = output.status.success();
            }
            Err(sys_err) => {
                result.system_err = sys_err;
                result.exit_code = -2;
            }
        }

        result
    }

    /// 在工程目录下执行自定义 `cargo <args>` 命令（如 `cargo test`、`cargo check`、
    /// `cargo clippy` 等），超时则终止并记为 [`SystemError::Timeout`]；
    /// 自身不 panic，失败均体现在返回的 [`DnyResult`] 中。
    ///
    /// 耗时记为 [`DnyResult::run_duration`]，[`DnyResult::build_duration`] 恒为 `Duration::ZERO`。
    ///
    /// # Note: 不透传 `is_release`
    ///
    /// 与 [`build`](Self::build) 不同，本方法**不会**根据 `is_release` 自动追加
    /// `--release`，`args` 将原样透传给 `cargo`。需要 release 行为时请调用者显式传入：
    /// `runner.cargo(&["test", "--release"], None)`。
    ///
    /// # 错误语义
    ///
    /// - `cargo` 进程启动失败 → [`SystemError::ProcessSpawnFailed`]；
    /// - 超时被终止 → [`SystemError::Timeout`]；
    /// - 正常退出但退出码非零（如 `check`/`test` 未通过）→ [`SystemError::CargoFailed`]，
    ///   此时 [`DnyResult::ok`] 为 `false`，`stdout`/`stderr`/`exit_code` 保留子进程输出供诊断；
    /// - 退出码为 0 → `ok == true`，`system_err == SystemError::None`。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lib_unknown::dyntest::DnyRun;
    ///
    /// let runner = DnyRun::new("fn main() {}", "");
    /// let res = runner.cargo(&["check"], None);
    /// assert!(res.ok);
    ///
    /// // release 行为不会自动追加，需显式传入：
    /// let res = runner.cargo(&["test", "--release"], None);
    /// assert!(res.ok);
    /// ```
    pub fn cargo(&self, args: &[&str], timeout: Option<Duration>) -> DnyResult {
        let mut result = DnyResult::default();
        let mut cmd = Command::new("cargo");
        cmd.args(args).current_dir(&self.dir);
        cmd.envs(&self.envs);

        match Self::execute_cmd(&mut cmd, timeout) {
            Ok((output, duration)) => {
                result.run_duration = duration;
                result.stdout = String::from_utf8_lossy(&output.stdout).to_string();
                result.stderr = String::from_utf8_lossy(&output.stderr).to_string();
                result.exit_code = output.status.code().unwrap_or(-1);
                result.ok = output.status.success();
                if !result.ok {
                    result.system_err = SystemError::CargoFailed;
                }
            }
            Err(sys_err) => {
                result.system_err = sys_err;
                result.exit_code = -2;
            }
        }
        result
    }

    /// 替换 `main.rs` 内容并同步到工程目录（供缓存复用时复写任务代码）。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Panics
    ///
    /// - 文件写入失败时 panic。
    pub fn reset_main_code(&mut self, main_code: &str) {
        self.main_code = main_code.to_string();
        self.sync_files();
    }
    /// 替换依赖声明并同步到工程目录。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Panics
    ///
    /// - 文件写入失败时 panic。
    pub fn reset_deps(&mut self, deps: &str) {
        self.deps = deps.to_string();
        self.sync_files();
    }
}

/// 工程心跳锁：在 `target/dyn_tests/lock/` 下创建锁文件并由后台线程每 500ms 续写，`Drop` 时停线程删文件，用于识别僵尸工程。
///
/// # Feature Requirement
///
/// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
pub struct DynTestLock {
    lock_path: PathBuf,
    stop_signal: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl DynTestLock {
    /// 创建锁文件并启动心跳线程。
    ///
    /// # Feature Requirement
    ///
    /// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
    ///
    /// # Panics
    ///
    /// - 无法获取当前目录、锁目录/锁文件创建失败，或心跳线程无法启动时 panic。
    pub fn new(id: &str) -> Self {
        let mut lock_path = std::env::current_dir().unwrap();
        lock_path.push("target");
        lock_path.push("dyn_tests");
        lock_path.push("lock");

        fs::create_dir_all(&lock_path).expect("无法创建锁目录");
        let lock_file = lock_path.join(format!("lock_{}", id));
        fs::File::create(&lock_file).expect("无法创建锁文件");

        let stop_signal = Arc::new(AtomicBool::new(false));
        let thread_stop_signal = stop_signal.clone();
        let thread_lock_file = lock_file.clone();

        let thread_handle = thread::spawn(move || {
            let mut counter = 0u64;
            let heartbeat_interval = Duration::from_millis(500);
            while !thread_stop_signal.load(Ordering::Relaxed) {
                if let Ok(mut file) = fs::File::create(&thread_lock_file) {
                    let _ = write!(file, "heartbeat: {}", counter);
                }
                counter += 1;
                for _ in 0..10 {
                    if thread_stop_signal.load(Ordering::Relaxed) {
                        break;
                    }
                    thread::sleep(heartbeat_interval / 10);
                }
            }
        });

        Self {
            lock_path: lock_file,
            stop_signal,
            thread_handle: Some(thread_handle),
        }
    }
}

impl Drop for DynTestLock {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        if self.lock_path.exists() {
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}

/// 清理全部动态测试工程：等待活跃锁释放（心跳停滞超 1500ms 的视为僵尸并清除其锁），再删除 `target/dyn_tests` 目录。
///
/// # Feature Requirement
///
/// 需要同时启用 `"dyntest"` 与 `"std"` 特性。
///
/// # Examples
///
/// ```rust,no_run
/// use lib_unknown::dyntest::clear_dny_project;
///
/// clear_dny_project(None);
/// ```
///
/// # Panics
///
/// - 无法获取当前目录时 panic；超时仅打印警告，不 panic。
pub fn clear_dny_project(timeout: Option<Duration>) {
    thread::sleep(Duration::from_secs_f32(0.3));
    let base_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("dyn_tests");
    let lock_dir = base_dir.join("lock");
    let start_time = Instant::now();
    let zombie_threshold = Duration::from_millis(1500);

    if lock_dir.exists() {
        loop {
            let mut active_locks = 0;
            if let Ok(entries) = fs::read_dir(&lock_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    let is_zombie = match fs::metadata(&path).and_then(|m| m.modified()) {
                        Ok(modified_time) => {
                            modified_time.elapsed().unwrap_or(Duration::ZERO) > zombie_threshold
                        }
                        Err(_) => true,
                    };

                    if is_zombie {
                        let _ = fs::remove_file(&path);
                    } else {
                        active_locks += 1;
                    }
                }
            }

            if active_locks == 0 {
                break;
            }
            if let Some(limit) = timeout
                && start_time.elapsed() >= limit
            {
                eprintln!("警告: 清除动态测试环境超时！");
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    if base_dir.exists() {
        let _ = fs::remove_dir_all(base_dir);
    }
}
