use miniserde::{Deserialize, Serialize, json};
use std::process::Stdio;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::process::Command;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemoteExec {
    message: String,
    task_id: String,
    command: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemoteExecCallback {
    task_id: String,
    result: String,
    exit_code: i32,
    finished_at: String,
}

pub async fn exec_command(remote_exec: RemoteExec, callback_url: &str) -> Result<(), String> {
    let exec = tokio::spawn(async move {
        let child = if let Ok(child) = Command::new("bash")
            .arg("-c")
            .arg(remote_exec.command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            child
        } else {
            return Err("failed to execute process".to_string());
        };

        let output = if let Ok(output) = child.wait_with_output().await {
            output
        } else {
            return Err("failed to get process output".to_string());
        };

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        let status = output.status.code().unwrap_or(1);

        Ok((status, format!("{stdout_str}\n{stderr_str}")))
    });

    let (status, output) = if let Ok(result) = exec.await {
        if let Ok(result) = result {
            result
        } else {
            return Err("failed to execute process".to_string());
        }
    } else {
        return Err("failed to execute process".to_string());
    };

    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let finished_at = now.format(&Rfc3339).unwrap_or_default();

    let reply = RemoteExecCallback {
        task_id: remote_exec.task_id,
        result: output,
        exit_code: status,
        finished_at,
    };

    if let Ok(req) = ureq::post(callback_url).send(&json::to_string(&reply)) {
        if req.status().is_success() {
            Ok(())
        } else {
            Err("server returned a error".to_string())
        }
    } else {
        Err("Unable to connect server".to_string())
    }
}
