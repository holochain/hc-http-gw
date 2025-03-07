use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
};

use anyhow::{bail, Context};
use regex::Regex;
use url2::Url2;

const HOST_URL: &str = "http://localhost";
const LAIR_PASSWORD: &str = "test-password";
const DEFAULT_TIMEOUT: u64 = 30000; // 30 seconds

pub struct Conductor {
    process: Option<Child>,
    conductor_dir: Option<String>,
    admin_api_url: Url2,
    timeout: u64,
}

impl Conductor {
    pub fn create(
        signaling_server_url: &str,
        network_type: &str,
        bootstrap_server_url: Option<&str>,
        timeout: Option<u64>,
    ) -> anyhow::Result<Self> {
        if bootstrap_server_url.is_some() && !matches!(network_type, "webrtc") {
            bail!("Error creating conductor: bootstrap service can only be set for webrtc network");
        }

        let mut args = vec![
            "sandbox",
            "--piped",
            "create",
            "--in-process-lair",
            "network",
        ];

        if let Some(bootstrap_url) = bootstrap_server_url {
            args.push("--bootstrap");
            args.push(bootstrap_url);
        }

        args.push(network_type);
        args.push(signaling_server_url);

        tracing::info!("Creating conductor with args {:?}", args);

        let mut create_process = Command::new("hc")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Write the Lair password to stdin
        if let Some(stdin) = create_process.stdin.as_mut() {
            stdin.write_all(LAIR_PASSWORD.as_bytes())?;
            stdin.flush()?;
        } else {
            bail!("Failed to open stdin for the conductor process");
        }

        // Initialize conductor with default values
        let mut conductor = Self {
            process: Some(create_process),
            conductor_dir: None,
            admin_api_url: Url2::parse(HOST_URL),
            timeout: timeout.unwrap_or(DEFAULT_TIMEOUT),
        };

        // Process stdout to extract the conductor directory
        if let Some(process) = &mut conductor.process {
            let stdout = process
                .stdout
                .take()
                .context("Failed to capture conductor process stdout")?;

            let reader = BufReader::new(stdout);
            let config_path_regex = Regex::new(r#"ConfigRootPath\("(.*?)"\)"#)?;

            for line in reader.lines() {
                let line = line?;
                tracing::debug!("Conductor output: {}", line);

                // Extract conductor directory path
                if let Some(captures) = config_path_regex.captures(&line) {
                    if let Some(path) = captures.get(1) {
                        conductor.conductor_dir = Some(path.as_str().to_string());
                        tracing::debug!("Found conductor directory: {:?}", conductor.conductor_dir);
                    }
                }

                // Check for completion or errors
                if line.contains("Conductor created") {
                    tracing::debug!("Conductor creation completed successfully");
                    break;
                }
            }

            // Check stderr for any errors
            if let Some(stderr) = process.stderr.take() {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    let line = line?;
                    tracing::error!("Conductor error: {}", line);
                }
            }

            // Wait for process to complete
            let status = process.wait()?;
            if !status.success() {
                bail!("Conductor creation failed with status: {}", status);
            }
        }

        // Clear the process field since it's now completed
        conductor.process = None;

        if conductor.conductor_dir.is_none() {
            bail!("Failed to determine conductor directory");
        }

        Ok(conductor)
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.conductor_dir.is_none() {
            bail!("Cannot start conductor: conductor has not been created");
        }

        if self.process.is_some() {
            tracing::error!("Cannot start conductor: conductor is already running");
            return Ok(());
        }

        let conductor_dir = self.conductor_dir.as_ref().unwrap();
        tracing::debug!("Starting conductor with directory: {}", conductor_dir);

        // Spawn the run process
        let mut run_process = Command::new("hc")
            .args(&["sandbox", "--piped", "run", "-e", conductor_dir])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Write the Lair password to stdin
        if let Some(stdin) = run_process.stdin.as_mut() {
            stdin.write_all(LAIR_PASSWORD.as_bytes())?;
            stdin.flush()?;
        } else {
            bail!("Failed to open stdin for the conductor process");
        }

        // Process stdout to extract the admin port
        let stdout = run_process
            .stdout
            .take()
            .context("Failed to capture conductor process stdout")?;

        let reader = BufReader::new(stdout);
        let port_regex = Regex::new(r"Conductor launched #!\d+ (\{.*\})")?;

        // Create a thread to process the output and extract port info
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in reader.lines() {
                if let Ok(line) = line {
                    tracing::info!("Conductor output: {}", line);

                    // Extract port configuration
                    if let Some(captures) = port_regex.captures(&line) {
                        if let Some(json_str) = captures.get(1) {
                            // match serde_json::from_str::<Value>(json_str.as_str()) {
                            //     Ok(port_config) => {
                            //         if let Some(admin_port) = port_config["admin_port"].as_u64() {
                            //             let _ = tx.send(admin_port as u16);
                            //         }
                            //     }
                            //     Err(e) => tracing::error!("Failed to parse port configuration: {}", e),
                            // }
                        }
                    }

                    // Check for successful startup
                    if line.contains("Conductor ready") {
                        break;
                    }
                }
            }
        });

        // Wait for the admin port to be received
        match rx.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(admin_port) => {
                self.admin_api_url.set_port(Some(admin_port)).unwrap();
                tracing::debug!("Conductor started with admin port: {}", admin_port);
            }
            Err(_) => {
                // Kill the process if we time out waiting for the port
                let _ = run_process.kill();
                bail!("Timed out waiting for conductor to start");
            }
        }

        // Store the process
        self.process = Some(run_process);

        Ok(())
    }

    /// Shut down the conductor process
    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        if let Some(mut process) = self.process.take() {
            tracing::debug!("Shutting down conductor");
            process.kill()?;
        }
        Ok(())
    }
}
