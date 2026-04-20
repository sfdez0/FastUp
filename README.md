[![Rust CI](https://github.com/sfdez0/FastUp/actions/workflows/rust.yml/badge.svg)](https://github.com/sfdez0/FastUp/actions/workflows/rust.yml) [![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](https://choosealicense.com/licenses/mit/)

# FastUp

FastUp is a CLI tool designed for Linux environments to streamline your local development workflow. Instead of manually opening multiple terminal tabs to start databases, APIs, or workers, you can define them in a single YAML configuration file and manage them with simple, intuitive commands.

Everything stays local; this tool focuses purely on your machine's processes, with no cloud dependencies or remote setups.

Why I built this?
This project is my first dive into Rust. I built it to learn the language from scratch as it serves as a practical, hands-on exercise to transition from knowing zero Rust to building a functional tool for my daily use.


## Run Locally

Clone the project

```bash
  git clone https://github.com/your-username/fastup.git
```

Go to the project directory

```bash
  cd fastup
```

Install the binary

```bash
  cargo install --path .
```


## Configuration
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
```
## Usage

`fastup up _name` Starts an element

`fastup down _name` Stops an element

`fastup status` Prints status of elements

## Author

- [@sfdez0](https://github.com/sfdez0)
