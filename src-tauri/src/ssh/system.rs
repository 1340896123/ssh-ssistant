use super::client::AppState;
use crate::ssh::{execute_ssh_operation, ssh2_retry};
use serde::{Deserialize, Serialize};
use std::io::{ErrorKind, Read};
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

fn extract_block(text: &str, marker: &str) -> String {
    let start_tag = format!("{}_START", marker);
    let end_tag = format!("{}_END", marker);

    if let Some(start) = text.find(&start_tag) {
        let content_start = start + start_tag.len();
        if let Some(end) = text[content_start..].find(&end_tag) {
            return text[content_start..content_start + end].trim().to_string();
        }
    }
    String::new()
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

    // The shell command script
    let command_script = r#"
    export LC_ALL=C;
    echo "UPTIME_START";
    (uptime -p 2>/dev/null || uptime 2>/dev/null);
    echo "UPTIME_END";
    
    echo "MOUNTS_START";
    df -Ph 2>/dev/null | awk 'NR>1 {print $1 "|" $2 "|" $3 "|" $4 "|" $5 "|" $6}';
    echo "MOUNTS_END";
    
    echo "IP_START";
    (hostname -I 2>/dev/null || echo 'n/a');
    echo "IP_END";
    
    echo "CPU_START";
    CPU1=$(grep '^cpu ' /proc/stat 2>/dev/null);
    sleep 0.1;
    CPU2=$(grep '^cpu ' /proc/stat 2>/dev/null);
    if [ -n "$CPU1" ] && [ -n "$CPU2" ]; then
        echo "$CPU1 $CPU2" | awk '{
            u1=$2+$4+$5; t1=$2+$4+$5+$6;
            u2=$8+$10+$11; t2=$8+$10+$11+$12;
            if (t2-t1 > 0) printf "%.1f", (u2-u1) * 100 / (t2-t1); else print "0"
        }';
    else
        top -bn1 2>/dev/null | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//' | sed 's/%id,.*//' || echo "0";
    fi;
    echo "";
    echo "CPU_END";
    
    echo "MEMORY_START";
    awk '/MemTotal:/ {total=$2} /MemAvailable:/ {avail=$2} END {if(total>0){used=total-avail; printf "%.1f%%|%.1fGB|%.1fGB|%.1fGB", (used/total)*100, total/1024/1024, used/1024/1024, avail/1024/1024} else {print "0%|0|0|0"}}' /proc/meminfo 2>/dev/null;
    echo ""; 
    echo "MEMORY_END";
    
    echo "PROCESSES_START";
    ps aux --sort=-%cpu --no-headers 2>/dev/null | head -5 | awk '{printf "%s|%s|%s|%s|%.1fMB\\n", $2, $11, $3"%", $4"%", $6/1024}';
    echo "PROCESSES_END";
    
    echo "MEMORY_PROCESSES_START";
    ps aux --sort=-%mem --no-headers 2>/dev/null | head -5 | awk '{printf "%s|%s|%s|%s|%.1fMB\\n", $2, $11, $3"%", $4"%", $6/1024}';
    echo "MEMORY_PROCESSES_END";
    "#.replace("\n", " ").trim().to_string();

    let output = execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;

        let sess = bg_session.lock().unwrap();
        let mut channel = ssh2_retry(|| sess.channel_session()).map_err(|e| e.to_string())?;

        ssh2_retry(|| channel.exec(&command_script)).map_err(|e| e.to_string())?;

        let mut s = String::new();
        let mut buf = [0u8; 4096];

        loop {
            match channel.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                    s.push_str(&chunk);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    return Err(e.to_string());
                }
            }
        }
        ssh2_retry(|| channel.wait_close())
            .map_err(|e| format!("Failed to wait for channel close: {}", e))?;

        Ok(s)
    })
    .await?;

    // Parse the output
    let uptime = extract_block(&output, "UPTIME");
    let mounts_str = extract_block(&output, "MOUNTS");
    let ip_str = extract_block(&output, "IP");
    let cpu_str = extract_block(&output, "CPU");
    let memory_str = extract_block(&output, "MEMORY");
    let proc_cpu_str = extract_block(&output, "PROCESSES");
    let proc_mem_str = extract_block(&output, "MEMORY_PROCESSES");

    let ip = ip_str
        .split_whitespace()
        .next()
        .unwrap_or("N/A")
        .to_string();

    // CPU
    let cpu_val = cpu_str.parse::<f64>().unwrap_or(0.0);
    let cpu_usage = format!("{:.1}%", cpu_val.clamp(0.0, 100.0));

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
        uptime: if uptime.is_empty() {
            "N/A".to_string()
        } else {
            uptime
        },
        disk: root_disk,
        mounts,
        ip,
        cpu: final_cpu,
        memory: final_memory,
    })
}
