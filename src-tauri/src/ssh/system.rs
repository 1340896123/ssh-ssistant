use super::client::{AppState, ClientType};
use crate::ssh::{execute_ssh_operation, SshCommand};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use tauri::{command, AppHandle, State};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub size: String,
    pub used: String,
    pub avail: String,
    pub percent: String,
    pub mount: String,
    pub filesystem: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: String,
    pub command: String,
    pub cpu: String,
    pub memory: String,
    pub memory_percent: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub usage: String,
    pub top_processes: Vec<ProcessInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub usage: String,
    pub total: String,
    pub used: String,
    pub available: String,
    pub top_processes: Vec<ProcessInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionStats {
    pub uptime: String,
    pub disk: Option<DiskInfo>,
    pub mounts: Vec<DiskInfo>,
    pub ip: String,
    pub cpu: Option<CpuInfo>,
    pub memory: Option<MemoryInfo>,
}

// Helper to run command on SSH session
// Helper to run command on SSH session
fn run_ssh_command(sender: &Sender<SshCommand>, cmd: &str) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    sender.send(SshCommand::Exec {
        command: cmd.to_string(),
        listener: tx,
        cancel_flag: None,
    }).map_err(|e| format!("Failed to send command: {}", e))?;
    
    rx.recv().map_err(|_| "Failed to receive response from SSH Manager".to_string())?
}

// Helper to run command on WSL
fn run_wsl_command(distro: &str, cmd: &str) -> Result<String, String> {
     let output = std::process::Command::new("wsl")
        .arg("-d")
        .arg(distro)
        .arg("bash")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| e.to_string())?;
    
    // We treat stderr as potential non-fatal or just mix it, but for stats we usually want clean output.
    // However, some commands might output to stderr on non-error (unlikely for these standard tools).
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        // If failed, return empty or error message?
        // Return empty string to allow fallback handling or partial stats
        Ok("".to_string()) 
    }
}

fn parse_table<T, F>(raw: &str, mapper: F, min_columns: usize) -> Vec<T>
where
    F: Fn(Vec<&str>) -> Option<T>,
{
    if raw.is_empty() {
        return Vec::new();
    }
    raw.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.split('|').collect::<Vec<&str>>())
        .filter(|parts| parts.len() >= min_columns)
        .filter_map(mapper)
        .collect()
}

fn parse_cpu_stats(line: &str) -> Option<(u64, u64)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 || parts[0] != "cpu" {
        return None;
    }
    // parts[0] is "cpu"
    // user: 1, nice: 2, system: 3, idle: 4, iowait: 5, irq: 6, softirq: 7, steal: 8
    let parse = |i| parts.get(i).and_then(|s: &&str| s.parse::<u64>().ok()).unwrap_or(0);

    let user = parse(1);
    let nice = parse(2);
    let system = parse(3);
    let idle = parse(4);
    let iowait = parse(5);
    let irq = parse(6);
    let softirq = parse(7);
    let steal = parse(8);

    let total = user + nice + system + idle + iowait + irq + softirq + steal;
    let work = user + nice + system + irq + softirq + steal;

    Some((total, work))
}

#[command]
pub async fn get_remote_system_status(
    _app_handle: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<SessionStats, String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    // Execute commands in steps
    let (uptime_str, mounts_str, ip_str, cpu_str, memory_str, proc_cpu_str, proc_mem_str) = match &client.client_type {
        ClientType::Ssh(sender) => {
            let sender = sender.clone();
            execute_ssh_operation(move || {
                // 1. Uptime
                let uptime = run_ssh_command(
                    &sender,
                    "export LC_ALL=C; (uptime -p 2>/dev/null || uptime 2>/dev/null)",
                )?;

                // 2. Mounts
                let mounts = run_ssh_command(
                    &sender,
                    "export LC_ALL=C; df -Ph 2>/dev/null | awk 'NR>1 {print $1 \"|\" $2 \"|\" $3 \"|\" $4 \"|\" $5 \"|\" $6}'",
                )?;

                // 3. IP
                let ip = run_ssh_command(
                    &sender,
                    "export LC_ALL=C; (hostname -I 2>/dev/null || echo 'n/a')",
                )?;

                // 4. CPU
                let cpu_stat1 = run_ssh_command(&sender, "cat /proc/stat | grep '^cpu '").ok();
                
                let cpu = if let Some(stat1) = cpu_stat1 {
                    thread::sleep(Duration::from_millis(500));
                    if let Ok(stat2) = run_ssh_command(&sender, "cat /proc/stat | grep '^cpu '") {
                         match (parse_cpu_stats(&stat1), parse_cpu_stats(&stat2)) {
                            (Some((t1, w1)), Some((t2, w2))) if t2 > t1 => {
                                let total_delta = t2 - t1;
                                let work_delta = w2 - w1;
                                let usage = (work_delta as f64 / total_delta as f64) * 100.0;
                                format!("{:.1}", usage)
                            }
                            _ => "0".to_string(),
                        }
                    } else {
                        "0".to_string()
                    }
                } else {
                     let top_cmd = "top -bn1 2>/dev/null | grep \"Cpu(s)\" | awk '{print $2}' | sed 's/%us,//' | sed 's/%id,.*//'";
                     run_ssh_command(&sender, top_cmd).unwrap_or_else(|_| "0".to_string())
                };

                // 5. Memory
                let mem_cmd = r#"export LC_ALL=C; awk '/MemTotal:/ {total=$2} /MemAvailable:/ {avail=$2} END {if(total>0){used=total-avail; printf "%.1f%%|%.1fGB|%.1fGB|%.1fGB", (used/total)*100, total/1024/1024, used/1024/1024, avail/1024/1024} else {print "0%|0|0|0"}}' /proc/meminfo 2>/dev/null"#;
                let memory = run_ssh_command(&sender, mem_cmd)?;

                // 6. Processes (CPU sorted)
                let proc_cpu_cmd = r#"export LC_ALL=C; ps aux --sort=-%cpu --no-headers 2>/dev/null | head -5 | awk '{printf "%s|%s|%s|%s|%.1fMB\n", $2, $11, $3"%", $4"%", $6/1024}'"#;
                let proc_cpu = run_ssh_command(&sender, proc_cpu_cmd)?;

                // 7. Processes (Memory sorted)
                let proc_mem_cmd = r#"export LC_ALL=C; ps aux --sort=-%mem --no-headers 2>/dev/null | head -5 | awk '{printf "%s|%s|%s|%s|%.1fMB\n", $2, $11, $3"%", $4"%", $6/1024}'"#;
                let proc_mem = run_ssh_command(&sender, proc_mem_cmd)?;

                Ok((uptime, mounts, ip, cpu, memory, proc_cpu, proc_mem))
            }).await?
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                // 1. Uptime
                let uptime = run_wsl_command(&distro, "export LC_ALL=C; (uptime -p 2>/dev/null || uptime 2>/dev/null)")?;
                
                // 2. Mounts
                let mounts = run_wsl_command(&distro, "export LC_ALL=C; df -Ph 2>/dev/null | awk 'NR>1 {print $1 \"|\" $2 \"|\" $3 \"|\" $4 \"|\" $5 \"|\" $6}'")?;
                
                // 3. IP
                let ip = run_wsl_command(&distro, "export LC_ALL=C; (hostname -I 2>/dev/null || echo 'n/a')")?;
                
                // 4. CPU
                let cpu_stat1 = run_wsl_command(&distro, "cat /proc/stat | grep '^cpu '").ok();
                let cpu = if let Some(stat1) = cpu_stat1 {
                    if stat1.is_empty() { "0".to_string() } else {
                        thread::sleep(Duration::from_millis(500));
                         if let Ok(stat2) = run_wsl_command(&distro, "cat /proc/stat | grep '^cpu '") {
                            match (parse_cpu_stats(&stat1), parse_cpu_stats(&stat2)) {
                                (Some((t1, w1)), Some((t2, w2))) if t2 > t1 => {
                                    let total_delta = t2 - t1;
                                    let work_delta = w2 - w1;
                                    let usage = (work_delta as f64 / total_delta as f64) * 100.0;
                                    format!("{:.1}", usage)
                                }
                                _ => "0".to_string(),
                            }
                        } else { "0".to_string() }
                    }
                } else { "0".to_string() };
                
                // 5. Memory
                let mem_cmd = r#"export LC_ALL=C; awk '/MemTotal:/ {total=$2} /MemAvailable:/ {avail=$2} END {if(total>0){used=total-avail; printf "%.1f%%|%.1fGB|%.1fGB|%.1fGB", (used/total)*100, total/1024/1024, used/1024/1024, avail/1024/1024} else {print "0%|0|0|0"}}' /proc/meminfo 2>/dev/null"#;
                let memory = run_wsl_command(&distro, mem_cmd)?;
                
                // 6. Processes
                let proc_cpu_cmd = r#"export LC_ALL=C; ps aux --sort=-%cpu --no-headers 2>/dev/null | head -5 | awk '{printf "%s|%s|%s|%s|%.1fMB\n", $2, $11, $3"%", $4"%", $6/1024}'"#;
                let proc_cpu = run_wsl_command(&distro, proc_cpu_cmd)?;
                
                let proc_mem_cmd = r#"export LC_ALL=C; ps aux --sort=-%mem --no-headers 2>/dev/null | head -5 | awk '{printf "%s|%s|%s|%s|%.1fMB\n", $2, $11, $3"%", $4"%", $6/1024}'"#;
                let proc_mem = run_wsl_command(&distro, proc_mem_cmd)?;
                
                Ok::<_, String>((uptime, mounts, ip, cpu, memory, proc_cpu, proc_mem))
            }).await.map_err(|e| format!("Task join error: {}", e))??
        }
    };

    // --- Parsing ---

    // IP
    let ip = ip_str
        .split_whitespace()
        .next()
        .unwrap_or("N/A")
        .to_string();

    // CPU
    let cpu_val = cpu_str.parse::<f64>().unwrap_or(0.0);
    let cpu_usage = format!("{:.1}%", cpu_val);

    // Memory
    let mem_parts: Vec<&str> = memory_str.split('|').collect();
    let memory_info = if mem_parts.len() >= 4 {
        Some(MemoryInfo {
            usage: mem_parts[0].to_string(),
            total: mem_parts[1].to_string(),
            used: mem_parts[2].to_string(),
            available: mem_parts[3].to_string(),
            top_processes: Vec::new(), // Will fill later
        })
    } else {
        None
    };

    // Mounts
    let mounts: Vec<DiskInfo> = parse_table(
        &mounts_str,
        |parts| {
            Some(DiskInfo {
                filesystem: parts[0].to_string(),
                size: parts[1].to_string(),
                used: parts[2].to_string(),
                avail: parts[3].to_string(),
                percent: parts[4].to_string(),
                mount: parts[5].to_string(),
            })
        },
        6,
    );

    let root_disk = mounts
        .iter()
        .find(|m| m.mount == "/")
        .cloned()
        .or_else(|| mounts.first().cloned());

    // Processes
    let process_mapper = |parts: Vec<&str>| {
        Some(ProcessInfo {
            pid: parts[0].to_string(),
            command: parts[1].to_string(),
            cpu: parts[2].to_string(),
            memory: parts[3].to_string(),
            memory_percent: parts[4].to_string(),
        })
    };

    let cpu_top_processes = parse_table(&proc_cpu_str, process_mapper, 5);
    let memory_top_processes = parse_table(&proc_mem_str, process_mapper, 5);

    let mut final_memory = memory_info;
    if let Some(ref mut m) = final_memory {
        m.top_processes = memory_top_processes;
    }

    let final_cpu = Some(CpuInfo {
        usage: cpu_usage,
        top_processes: cpu_top_processes,
    });

    Ok(SessionStats {
        uptime: if uptime_str.is_empty() {
            "N/A".to_string()
        } else {
            uptime_str
        },
        disk: root_disk,
        mounts,
        ip,
        cpu: final_cpu,
        memory: final_memory,
    })
}
