[![Build](https://github.com/sfdez0/FastUp/actions/workflows/build.yml/badge.svg)](https://github.com/sfdez0/FastUp/actions/workflows/build.yml) [![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](https://choosealicense.com/licenses/mit/)

# FastUp

FastUp is a CLI tool designed for Linux environments to streamline your local development workflow. Instead of manually opening multiple terminal tabs to start databases, APIs, or workers, you can define them in a single YAML configuration file and manage them with simple, intuitive commands.

Everything stays local; this tool focuses purely on your machine's processes, with no cloud dependencies or remote setups.

> **Why I built this?**
> This project is my first dive into Rust. I built it to learn the language from scratch as it serves as a practical, hands-on exercise to transition from knowing zero Rust to building a functional tool for my daily use.


## Run Locally

Clone the project:
```bash
git clone https://github.com/sfdez0/FastUp.git
```

Go to the project directory:

```bash
cd FastUp
```

Install the binary:

```bash
cargo install --path .
```


## Configuration

FastUp manages its configuration and logs through standard Linux directories:

| Purpose | Path |
|---------|------|
| Configuration file | `~/.config/fastup/fastup.yaml` |
| Logs directory | `~/.local/share/fastup/logs` |
| Main log file | `~/.local/share/fastup/logs/fastup.txt` |

The configuration file is where you define all your services, commands, and groups. Logs are automatically created when you run commands, helping you troubleshoot and monitor activity.

```yaml
# Example configuration file
elements_config:
  - name: "postgresql"
    port: 5432
    element_type: "Service"
    service_name: "postgresql"

  - name: "mysql"
    port: 3306
    element_type: "Service"
    service_name: "mysql"

  - name: "http_server"
    port: 8081
    element_type: "Command"
    start_command: "python3"
    args: 
      - "-m"
      - "http.server"
      - "8081"

  - name: "api"
    port: 8084
    element_type: "Command"
    start_command: "/home/u1/my_api/venv/bin/python"
    args: 
      - "-m"
      - "uvicorn"
      - "main:app"
      - "--app-dir"
      - "/home/u1/my_api"
      - "--host"
      - "0.0.0.0"
      - "--port"
      - "8084"
    log_file: "logs/fastapi.log"

groups_config:
  - name: "example_group_1"
    description: "Example group 1..."
    elements:
      - "postgresql"
      - "api"
  - name: "example_group_db"
    description: "Example group db..."
    elements:
      - "postgresql"
      - "mysql"
```
## Usage

The general syntax for FastUp is:

```bash
fastup <COMMAND> [OPTIONS] <NAME>
```

### Commands

* **`up`** — Starts a configured element.
  ```bash
  fastup up api                  # Start a single element
  fastup up -g example_group_db  # Start a group of elements
  ```

* **`down`** — Stops a running element.
  ```bash
  fastup down api                  # Stop a single element
  fastup down -g example_group_db  # Stop a group of elements
  ```

* **`status`** — Prints the current status of all elements.
  ```bash
  fastup status
  ```

### Global Options

* `-g` : Specifies that the `<NAME>` argument is a group (defined in `groups_config`).
* `-h, --help` : Prints help information for the command.
* `-V, --version` : Prints the current version of FastUp.

## Author

- [@sfdez0](https://github.com/sfdez0)
