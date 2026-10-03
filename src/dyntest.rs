// lib/src/dyntest.rs
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

#[derive(Debug, Clone, PartialEq)]
pub enum SystemError {
    None,
    Timeout(Duration),
    CompileFailed,
    ProcessSpawnFailed(String),
}

#[derive(Debug, Clone)]
pub struct DnyResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub build_duration: Duration,
    pub run_duration: Duration,
    pub ok: bool,
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

#[derive(Clone)]
pub struct DnyTask {
    pub tag: String,
    pub main_code: String,
    pub deps: String,
    pub envs: HashMap<String, String>,
    pub cargo_config: Option<String>,
    pub is_release: bool,
}

impl DnyTask {
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
    pub fn with_deps(mut self, deps: impl Into<String>) -> Self {
        self.deps = deps.into();
        self
    }
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.insert(key.into(), value.into());
        self
    }
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
    pub fn with_cargo_config(mut self, config: impl Into<String>) -> Self {
        self.cargo_config = Some(config.into());
        self
    }
    // [新增] 开启或关闭 release 模式
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
    pub fn new() -> Self {
        Self {
            timeout: None,
            only_build: false,
            use_cache: false,
            on_result: None,
        }
    }

    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout = Some(d);
        self
    }
    pub fn only_build(mut self, b: bool) -> Self {
        self.only_build = b;
        self
    }
    pub fn use_cache(mut self, c: bool) -> Self {
        self.use_cache = c;
        self
    }
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

pub struct DnyRun {
    pub main_code: String,
    pub deps: String,
    dir: PathBuf,
    project_name: String,
    lock: DynTestLock,
    envs: HashMap<String, String>,
    cargo_config: Option<String>,
    pub is_release: bool,
}

impl DnyRun {
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

    pub fn release(&mut self, release: bool) -> &mut Self {
        self.is_release = release;
        self
    }

    pub fn env(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.envs.insert(key.into(), value.into());
        self
    }

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

    pub fn clear_envs(&mut self) -> &mut Self {
        self.envs.clear();
        self
    }

    pub fn cargo_config(&mut self, config: impl Into<String>) -> &mut Self {
        self.cargo_config = Some(config.into());
        self.sync_cargo_config();
        self
    }

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

    pub fn run(&self, timeout: Option<Duration>) -> DnyResult {
        let build_result = self.build(timeout);
        if !build_result.ok {
            return build_result;
        }
        self.run_no_build(timeout, Some(build_result))
    }

    pub fn run_no_build(
        &self,
        timeout: Option<Duration>,
        build_result: Option<DnyResult>,
    ) -> DnyResult {
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
        let bin_path = target_dir.join(profile_dir).join(&exe_name);

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

    pub fn reset_main_code(&mut self, main_code: &str) {
        self.main_code = main_code.to_string();
        self.sync_files();
    }
    pub fn reset_deps(&mut self, deps: &str) {
        self.deps = deps.to_string();
        self.sync_files();
    }
}

pub struct DynTestLock {
    lock_path: PathBuf,
    stop_signal: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl DynTestLock {
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
