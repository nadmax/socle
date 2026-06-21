<h1 align="center">YAIMA</h1>
<p align="center">
    <strong>Yet Another Identity Management API</strong><br/>
    <em>Secure • Role-based • Written in Rust</em>
</p>

<p align="center">
    <a href="https://github.com/nadmax/yaima/actions">
        <img alt="CI" src="https://img.shields.io/github/actions/workflow/status/nadmax/yaima/ci.yaml?label=CI&logo=github"/>
    </a>
    <a href="https://opensource.org/licenses/MIT">
        <img alt="License" src="https://img.shields.io/github/license/nadmax/yaima"/>
    </a>
</p>

## Prerequisites

Ensure the following tools are installed:

* Rust
* Make
* Docker
* `sqlx-cli`
* `prek`

```sh
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Install prek
cargo install prek
```

## Getting Started

```sh
# Clone the repository
git clone https://github.com/nadmax/yaima.git
cd yaima

# Install Git hooks
make prek-install

# Configure .env file
cp .env.example .env

# Start Postgres and Valkey containers
make docker-up

# Run database migrations
make migrate

# Prepare SQLx offline metadata
make prepare

# Run the project
make dev
```

Docs will be available at [http://localhost:8080/apidocs](http://localhost:8080/apidocs)

## License

This project is licensed under the **MIT License**.

See the [LICENSE](https://github.com/nadmax/yaima/blob/master/LICENSE) file for details.
