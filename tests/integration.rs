use predicates::prelude::*;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

fn start_mock_ollama() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock Ollama");
    let addr = listener.local_addr().expect("get local addr");
    let handle = thread::spawn(move || {
        for attempt in 0..2 {
            let (mut stream, _peer) = listener.accept().expect("accept connection");
            let mut reader = BufReader::new(&mut stream);
            let mut headers = String::new();

            loop {
                let mut line = String::new();
                reader.read_line(&mut line).expect("read request line");
                if line == "\r\n" {
                    break;
                }
                headers.push_str(&line);
            }

            let content_length = headers
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);

            let mut body = vec![0u8; content_length];
            reader
                .read_exact(&mut body)
                .expect("read request body");

            let response_text = if attempt == 0 {
                r#"{"action":"call_tool","tool_name":"shell","tool_input":"echo end_to_end"}"#
            } else {
                r#"{"action":"final_answer","answer":"integration complete"}"#
            };

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_text.len(), response_text
            );
            stream
                .write_all(response.as_bytes())
                .expect("write mock response");
        }
    });

    (format!("http://{}", addr), handle)
}

fn write_config(temp_dir: &PathBuf, mock_url: &str) -> PathBuf {
    let config_path = temp_dir.join("config.toml");
    let config_contents = format!(r#"
port = 0
storage_backend = "sqlite"
database_url = "{}"
queue_backend = "inmemory"
encryption_key = ""
log_level = "info"
redis_url = ""
jwt_secret = "test_secret"
rabbitmq_url = ""
ollama_url = "{}"

agents = [
  {{ name = "planner", role = "planner", description = "Creates task plans." }},
  {{ name = "executor", role = "executor", description = "Executes plans." }},
  {{ name = "reviewer", role = "reviewer", description = "Reviews execution results." }},
  {{ name = "coder", role = "coder", description = "Writes code." }},
]
"#, temp_dir.join("ai_gateway_integration.db").display(), mock_url);
    fs::write(&config_path, config_contents).expect("write config file");
    config_path
}

#[test]
fn full_agent_loop_with_mock_ollama_and_shell_tool() {
    let (mock_url, server_handle) = start_mock_ollama();
    let temp_dir = env::temp_dir().join(format!("ai-gateway-integration-{}", Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).expect("create temp dir");

    let config_path = write_config(&temp_dir, &mock_url);

    let mut cmd = assert_cmd::Command::cargo_bin("ai-gateway").expect("binary exists");
    cmd.env("AI_GATEWAY_OLLAMA_URL", &mock_url)
        .arg("--config")
        .arg(&config_path)
        .arg("chat")
        .arg("Run the full agent flow")
        .timeout(Duration::from_secs(30));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Tool result: shell =>"))
        .stdout(predicate::str::contains("Final answer: integration complete"));

    server_handle.join().expect("mock server thread exited");
}
