# SQL Server executor fixture

This isolated fixture checks connection execution with the shared executor. It does not establish a ROM Storage provider.
The SQL Server 2025 RTM-CU9 server reports version 17.0.5005.3, Enterprise Developer Edition, on Ubuntu 24.04.4 LTS.
Developer edition is used only for development and testing.

Container: `rom-extras-mssql-20261008`. Named volume: `rom-extras-mssql-20261008`.
Resources: two CPUs, 4 GiB container memory, 3072 MiB SQL Server memory setting.
Endpoint: `127.0.0.1:55440`; database: `rom_extras_tests`.
Executed image: `mcr.microsoft.com/mssql/server@sha256:2b5b581621126574f3d1f75e78d3eebe8d05aedb59ad0cfdf9aa42cb0634d726`.
The initial moving tag was used only to discover the image digest.

Credentials are randomly generated. Local files `.superpowers/mssql.env` and `.superpowers/mssql-access.sh` are ignored and have mode 0600.
Do not commit or print those files. The fixture never uses user or production database credentials.

Run the dedicated checks with the local password variable:

```sh
source .superpowers/mssql-access.sh
./scripts/check-mssql
```

The driver is Tiberius 0.13.0, with default features disabled and `tds73` plus `rustls` enabled.
The fixture requires encrypted traffic but accepts its synthetic certificate on a fixed loopback endpoint.
This is not a production TLS validation profile. Production CA, hostname, and expired-certificate tests remain pending.

Connection and SQL fixture deadlines are eight seconds. Server lock timeout is five seconds.
Delayed durability is disabled. The fixture requests `XACT_ABORT ON`.
Each scenario creates a unique table and preserves it in the named volume.
Missing configuration fails tests. Storage receipts, journal, Work, revision races, and owner fencing remain pending.
